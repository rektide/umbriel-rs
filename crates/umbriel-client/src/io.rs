//! Cancel-safe readiness waits followed by synchronous, fully accounted attempts.
use tokio::net::UnixStream;
use umbriel_protocol::codec::Framer;
use umbriel_session::{EndReason, ProtocolFailure, Session};

pub(crate) fn end(session: &mut Session, reason: EndReason) {
    match reason {
        EndReason::Protocol(failure) => {
            session.protocol_failed(failure);
        }
        EndReason::Eof => {
            session.eof();
        }
        EndReason::LocalClose => {
            session.close();
        }
        other => {
            session.transport_lost(format!("{other:?}"));
        }
    }
}

/// One bounded I/O turn. Read size leaves enough room for a maximum frame's LF.
/// WouldBlock acknowledges zero; no write future may outlive begin_write.
pub(crate) async fn step(
    socket: &UnixStream,
    session: &mut Session,
    framer: &mut Framer,
    buffered_limit: usize,
) -> Result<bool, EndReason> {
    tokio::select! {
        result = socket.writable(), if !session.outbound().is_empty() => {
            result.map_err(lost)?;
            let bytes = session.begin_write().expect("single write owner");
            match socket.try_write(bytes) {
                Ok(0) => Err(lost(std::io::Error::from(std::io::ErrorKind::WriteZero))),
                Ok(count) => { session.write_progress(count).expect("actual write count"); Ok(true) },
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => { session.write_progress(0).expect("zero progress"); Ok(true) },
                Err(error) => Err(lost(error)),
            }
        },
        result = socket.readable() => {
            result.map_err(lost)?;
            let mut bytes = [0; 8192];
            let size = bytes.len().min(buffered_limit - framer.buffered_len());
            if size == 0 { return Err(EndReason::Protocol(ProtocolFailure::Framing("input budget exhausted".into()))); }
            match socket.try_read(&mut bytes[..size]) {
                Ok(0) => {
                    framer.finish().map_err(|e| EndReason::Protocol(ProtocolFailure::Framing(e.to_string())))?;
                    Ok(false)
                },
                Ok(count) => {
                    framer.push(&bytes[..count]).map_err(|e| EndReason::Protocol(ProtocolFailure::Framing(e.to_string())))?;
                    Ok(true)
                },
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(true),
                Err(error) => Err(lost(error)),
            }
        }
    }
}
fn lost(error: std::io::Error) -> EndReason {
    EndReason::TransportLost(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use umbriel_protocol::{codec::Limits, request::Request};

    #[tokio::test]
    async fn actual_short_writes_and_wouldblock_preserve_suffix() {
        let (socket, mut peer) = UnixStream::pair().unwrap();
        socket2::SockRef::from(&socket)
            .set_send_buffer_size(1024)
            .unwrap();
        let text = "x".repeat(60000);
        let mut session = Session::one_shot(1024);
        session.admit(Request::Msg(&text)).unwrap();
        let expected = session.outbound().to_vec();
        let mut framer = Framer::new(Limits {
            frame: 1024,
            buffered: 1025,
        });
        step(&socket, &mut session, &mut framer, 1025)
            .await
            .unwrap();
        assert!(
            !session.outbound().is_empty(),
            "small Unix send buffer forces a short write"
        );
        assert!(session.outbound().len() < expected.len());
        loop {
            let offered = session.begin_write().unwrap();
            match socket.try_write(offered) {
                Ok(count) => {
                    session.write_progress(count).unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    let suffix = session.outbound().to_vec();
                    session.write_progress(0).unwrap();
                    assert_eq!(session.outbound(), suffix);
                    break;
                }
                Err(error) => panic!("unexpected write: {error}"),
            }
        }
        let length = expected.len();
        let reader = tokio::spawn(async move {
            let mut result = Vec::new();
            let mut bytes = [0; 113];
            while result.len() < length {
                let count = peer.read(&mut bytes).await.unwrap();
                assert!(count > 0);
                result.extend_from_slice(&bytes[..count]);
            }
            result
        });
        while !session.outbound().is_empty() {
            step(&socket, &mut session, &mut framer, 1025)
                .await
                .unwrap();
        }
        assert_eq!(reader.await.unwrap(), expected);
    }
}
