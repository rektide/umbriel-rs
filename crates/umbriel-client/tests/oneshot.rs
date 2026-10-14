use std::{ffi::OsString, os::unix::ffi::OsStringExt, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};
use umbriel_client::{Discovery, Endpoint, Error, Options};
use umbriel_protocol::request::{Query, Request};

#[test]
fn discovery_precedence_and_unix_bytes() {
    let mut config = Discovery {
        explicit: Some("/explicit".into()),
        umbriel_socket: Some("/env".into()),
        runtime_dir: Some("/run/".into()),
        wayland_display: Some(OsString::from_vec(b"/display\xff".to_vec())),
    };
    assert_eq!(
        config.resolve().unwrap().path(),
        std::path::Path::new("/explicit")
    );
    config.explicit = None;
    assert_eq!(
        config.resolve().unwrap().path(),
        std::path::Path::new("/env")
    );
    config.umbriel_socket = Some("".into());
    assert_eq!(
        config.resolve().unwrap().path().as_os_str(),
        &OsString::from_vec(b"/run//umbriel-/display\xff.sock".to_vec())
    );
    config.wayland_display = None;
    assert!(config.resolve().is_err());
}

async fn exchange(parts: Vec<&'static [u8]>) -> Result<umbriel_client::RawReply, Error> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    let peer = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "{\"cmd\":\"submap\"}\n");
        for part in parts {
            stream.get_mut().write_all(part).await.unwrap();
            tokio::task::yield_now().await;
        }
    });
    let result = Endpoint::new(path)
        .request(Request::Query(Query::Submap), &Options::default())
        .await;
    peer.await.unwrap();
    result
}

#[tokio::test]
async fn real_split_reply_and_remote_error() {
    let reply = exchange(vec![b"{\"ok\":", b"null}", b"\n"]).await.unwrap();
    assert_eq!(reply.decode::<Option<String>>().unwrap(), None);
    assert!(matches!(reply.decode::<bool>(), Err(Error::Decode(_))));
    let reply = exchange(vec![b"{\"err\":\"nope\"}\n"]).await.unwrap();
    assert!(matches!(reply.decode::<()>(), Err(Error::Remote(text)) if text == "nope"));
}

#[tokio::test]
async fn eof_and_truncated_tail_are_not_replies() {
    assert!(
        matches!(exchange(vec![]).await, Err(Error::Ended { terminal, reply_observed: false }) if terminal.unresolved.unwrap().possibly_sent)
    );
    assert!(matches!(
        exchange(vec![b"{\"ok\":null}\n{\"ok\":"]).await,
        Err(Error::Ended {
            reply_observed: true,
            ..
        })
    ));
}

#[tokio::test(start_paused = true)]
async fn deadline_retains_transmission_uncertainty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    let peer = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        tokio::time::sleep(Duration::from_secs(60)).await;
    });
    let result = Endpoint::new(path)
        .request(Request::Query(Query::Submap), &Options::default())
        .await;
    assert!(matches!(
        result,
        Err(Error::TimedOut {
            possibly_sent: true,
            ..
        })
    ));
    peer.abort();
}
