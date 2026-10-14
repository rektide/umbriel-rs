//! Exercise the shipped binary against scripted, strictly read-only Unix peers.
use std::{
    ffi::OsString,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
};
use umbriel_protocol::{codec, request::Query};

const BIN: &str = env!("CARGO_BIN_EXE_umbriel-dump-desktop");

fn fixture(query: Query) -> &'static str {
    match query {
        Query::Windows => include_str!("../../umbriel-protocol/tests/fixtures/models/windows.json"),
        Query::Workspaces => {
            include_str!("../../umbriel-protocol/tests/fixtures/models/workspaces.json")
        }
        Query::Submap => "null",
        Query::Layers => include_str!("../../umbriel-protocol/tests/fixtures/models/layers.json"),
        Query::Color => include_str!("../../umbriel-protocol/tests/fixtures/models/color.json"),
        Query::Tearing => include_str!("../../umbriel-protocol/tests/fixtures/models/tearing.json"),
        Query::KeyboardLayouts => {
            include_str!("../../umbriel-protocol/tests/fixtures/models/keyboard-layouts.json")
        }
    }
}

fn compact(json: &str) -> String {
    sonic_rs::to_string(&codec::decode::<Value>(json.as_bytes()).unwrap()).unwrap()
}

#[derive(Default)]
struct Script {
    replacement: Option<(Query, &'static str)>,
    fails_at_query: bool,
    empty_inventories: bool,
    missing_overview: bool,
    invalid_theme: bool,
    lose_subscription: bool,
}

async fn run(script: Script) -> Output {
    run_with(script, Vec::new()).await
}

async fn run_with(script: Script, extra_args: Vec<OsString>) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scripted.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let peer = tokio::spawn(async move {
        for query in Query::ALL {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert_eq!(line, format!("{{\"cmd\":\"{}\"}}\n", query.as_str()));
            let replacement = script.replacement.filter(|(q, _)| *q == query);
            let response = if let Some((_, response)) = replacement {
                response.to_owned()
            } else {
                let payload = if script.empty_inventories
                    && matches!(query, Query::Windows | Query::Workspaces | Query::Layers)
                {
                    "[]"
                } else {
                    fixture(query)
                };
                format!("{{\"ok\":{}}}", compact(payload))
            };
            stream
                .get_mut()
                .write_all(format!("{response}\n").as_bytes())
                .await
                .unwrap();
            // Dropping the one-shot peer supplies the required clean EOF.
            if replacement.is_some() && script.fails_at_query {
                return;
            }
        }
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(
            line,
            "{\"cmd\":\"subscribe\",\"events\":[\"theme\",\"overview\"]}\n"
        );
        if script.lose_subscription {
            return;
        }
        let theme = if script.invalid_theme {
            "null".to_owned()
        } else {
            compact(include_str!(
                "../../umbriel-protocol/tests/fixtures/models/theme.json"
            ))
        };
        let mut events = format!("{{\"event\":\"theme\",\"data\":{theme}}}\n");
        if !script.missing_overview {
            // Duplicate theme must not replace the first or complete the capture.
            events.push_str(&format!("{{\"event\":\"theme\",\"data\":{theme}}}\n"));
            events.push_str(
                "{\"event\":\"overview\",\"data\":{\"open\":true,\"future_field\":42}}\n",
            );
        }
        stream.get_mut().write_all(events.as_bytes()).await.unwrap();
        // Success, timeout, and decode failure must all close the subscription.
        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).await.unwrap();
        assert!(rest.is_empty(), "capture must never issue desktop actions");
    });
    let output = tokio::task::spawn_blocking(move || invoke_with(path, extra_args))
        .await
        .unwrap();
    peer.await.unwrap();
    output
}

fn invoke(path: PathBuf) -> Output {
    invoke_with(path, Vec::new())
}

fn invoke_with(path: PathBuf, extra_args: Vec<OsString>) -> Output {
    Command::new(BIN)
        .arg("--socket")
        .arg(path)
        .args(["--timeout-ms", "500"])
        .args(extra_args)
        .output()
        .unwrap()
}

fn dump(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    codec::decode(&output.stdout).unwrap()
}

#[tokio::test]
async fn captures_all_available_state_and_preserves_unknown_fields() {
    let value = dump(run(Script::default()).await);
    assert_eq!(value["format"].as_str(), Some("umbriel-desktop-dump"));
    assert_eq!(value["schema_version"].as_u64(), Some(1));
    assert_eq!(value["atomic"].as_bool(), Some(false));
    assert!(
        value.get("outputs").is_none(),
        "Wayland inspection must be explicitly requested"
    );
    assert_eq!(value["queries"].as_object().unwrap().len(), 7);
    assert_eq!(value["events"].as_object().unwrap().len(), 2);
    for query in Query::ALL {
        assert_eq!(
            value["queries"][query.as_str()]["status"].as_str(),
            Some("ok")
        );
        assert_eq!(
            value["queries"][query.as_str()]["data"],
            codec::decode::<Value>(fixture(query).as_bytes()).unwrap()
        );
    }
    let window = &value["queries"]["windows"]["data"][1];
    assert_eq!(window["pid"].as_i64(), Some(-1));
    assert_eq!(window["workspace"].as_str(), Some(""));
    assert!(window["future_field"].is_object());
    assert_eq!(
        value["queries"]["windows"]["data"][0]["x"].as_i64(),
        Some(-900)
    );
    assert!(value["queries"]["submap"]["data"].is_null());
    assert_eq!(
        value["events"]["overview"]["data"]["future_field"].as_u64(),
        Some(42)
    );
    assert!(!value["limitations"].as_array().unwrap().is_empty());
    assert_eq!(
        value["layout_shape"]["exact_layout_tree"].as_bool(),
        Some(false)
    );
    assert_eq!(
        value["layout_shape"]["workspaces"]
            .as_array()
            .unwrap()
            .len(),
        value["queries"]["workspaces"]["data"]
            .as_array()
            .unwrap()
            .len()
    );
    assert_eq!(
        value["layout_shape"]["scratchpads"]["synthetic-pad"],
        sonic_rs::json!([1])
    );
    for section in ["queries", "events"] {
        for (_, capture) in value[section].as_object().unwrap().iter() {
            let start = capture["started_elapsed_ms"].as_u64().unwrap();
            let end = capture["received_elapsed_ms"].as_u64().unwrap();
            assert!(start <= end && end <= value["elapsed_ms"].as_u64().unwrap());
        }
    }
}

#[tokio::test]
async fn known_empty_inventories_are_successful_snapshots() {
    let value = dump(
        run(Script {
            empty_inventories: true,
            ..Script::default()
        })
        .await,
    );
    for name in ["windows", "workspaces", "layers"] {
        assert_eq!(value["queries"][name]["status"].as_str(), Some("ok"));
        assert!(
            value["queries"][name]["data"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    assert!(
        value["layout_shape"]["workspaces"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn absent_keyboard_is_explicitly_unavailable_not_an_empty_snapshot() {
    let value = dump(
        run(Script {
            replacement: Some((Query::KeyboardLayouts, "{\"err\":\"no keyboard\"}")),
            ..Script::default()
        })
        .await,
    );
    assert_eq!(
        value["queries"]["keyboard-layouts"]["status"].as_str(),
        Some("unavailable")
    );
    assert_eq!(
        value["queries"]["keyboard-layouts"]["reason"].as_str(),
        Some("no keyboard")
    );
    assert!(value["queries"]["keyboard-layouts"].get("data").is_none());
}

fn failed(output: Output) {
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "do not publish incomplete captures"
    );
    assert!(!output.stderr.is_empty());
}

#[tokio::test]
async fn remote_errors_and_invalid_payloads_fail_without_partial_stdout() {
    for replacement in [
        (Query::Windows, "{\"err\":\"denied\"}"),
        (Query::Windows, "{\"ok\":[{\"id\":\"incomplete\"}]}"),
        (Query::KeyboardLayouts, "{\"err\":\"unexpected failure\"}"),
    ] {
        failed(
            run(Script {
                replacement: Some(replacement),
                fails_at_query: true,
                ..Script::default()
            })
            .await,
        );
    }
}

#[tokio::test]
async fn missing_invalid_or_lost_initial_events_fail_and_close_source() {
    for script in [
        Script {
            missing_overview: true,
            ..Script::default()
        },
        Script {
            invalid_theme: true,
            ..Script::default()
        },
        Script {
            lose_subscription: true,
            ..Script::default()
        },
    ] {
        failed(run(script).await);
    }
}

#[test]
fn cli_help_version_and_argument_validation_need_no_compositor() {
    let help = Command::new(BIN).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("not a complete restore recipe"));
    assert!(
        Command::new(BIN)
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success()
    );
    for args in [
        vec!["--socket"],
        vec!["--timeout-ms", "0"],
        vec!["--unknown"],
        vec!["--outputs-tool", "/path/to/cli"],
        vec!["--outputs-display"],
    ] {
        failed(Command::new(BIN).args(args).output().unwrap());
    }
}

#[test]
fn connection_failure_has_no_partial_stdout() {
    let dir = tempfile::tempdir().unwrap();
    failed(invoke(dir.path().join("absent.sock")));
}

fn output_tool(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("output tool with spaces");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
        [ \"$#\" = 2 ] || exit 90\n\
        [ \"$1\" = outputs ] || exit 91\n\
        [ \"$2\" = --json ] || exit 92\n\
        [ \"$WAYLAND_DISPLAY\" = 'fake display' ] || exit 93\n\
        [ -z \"${{UMBRIEL_SOCKET+x}}\" ] || exit 94\n\
        {body}\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn output_args(path: &Path) -> Vec<OsString> {
    vec![
        "--outputs-display".into(),
        "fake display".into(),
        "--outputs-tool".into(),
        path.as_os_str().into(),
    ]
}

#[tokio::test]
async fn opt_in_output_capture_is_read_only_explicitly_scoped_and_lossless() {
    let dir = tempfile::tempdir().unwrap();
    let tool = output_tool(
        dir.path(),
        "printf '%s\\n' '[{\"name\":\"MON-1\",\"position\":{\"x\":-1080,\"y\":0},\"scale\":1.5,\"transform\":\"90\",\"modes\":[{\"width\":1920,\"height\":1080,\"refresh_mhz\":60000,\"current\":true}],\"future_field\":42}]'\nprintf 'tool warning' >&2",
    );
    let value = dump(run_with(Script::default(), output_args(&tool)).await);
    let outputs = &value["outputs"];
    assert_eq!(outputs["status"].as_str(), Some("ok"));
    assert_eq!(outputs["wayland_display"].as_str(), Some("fake display"));
    assert_eq!(outputs["same_compositor_verified"].as_bool(), Some(false));
    assert_eq!(outputs["tool_stderr"].as_str(), Some("tool warning"));
    assert_eq!(outputs["data"][0]["position"]["x"].as_i64(), Some(-1080));
    assert_eq!(outputs["data"][0]["scale"].as_f64(), Some(1.5));
    assert_eq!(outputs["data"][0]["future_field"].as_u64(), Some(42));
    assert!(
        outputs["received_elapsed_ms"].as_u64().unwrap() <= value["elapsed_ms"].as_u64().unwrap()
    );
}

#[tokio::test]
async fn requested_output_capture_failures_do_not_publish_partial_json() {
    let dir = tempfile::tempdir().unwrap();
    for body in [
        "printf 'denied' >&2\nexit 3",
        "printf 'not json'",
        "printf '{}'",
        "printf '[1]'",
        // exec ensures the bounded helper itself, not a background descendant, hangs.
        "exec sleep 10",
        "exec head -c 9000000 /dev/zero",
        "exec head -c 70000 /dev/zero >&2",
    ] {
        let tool = output_tool(dir.path(), body);
        failed(run_with(Script::default(), output_args(&tool)).await);
    }
    failed(
        run_with(
            Script::default(),
            output_args(&dir.path().join("absent tool")),
        )
        .await,
    );
}
