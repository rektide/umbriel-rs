//! Opt-in reuse of the installed read-only Wayland output inspection command.
use std::{ffi::OsStr, io, path::Path, process::Stdio, time::Duration};

use serde::Serialize;
use sonic_rs::{JsonValueTrait, Value};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    time::{Instant, timeout},
};
use umbriel_protocol::codec;

use super::ToolResult;

#[derive(Serialize)]
pub struct OutputCapture {
    provider: &'static str,
    tool: String,
    wayland_display: String,
    runtime_dir: Option<String>,
    /// A caller-selected Wayland display cannot be identified from an IPC path.
    same_compositor_verified: bool,
    started_elapsed_ms: u128,
    received_elapsed_ms: u128,
    status: &'static str,
    data: Value,
    #[serde(skip_serializing_if = "String::is_empty")]
    tool_stderr: String,
}

pub async fn capture(
    tool: &Path,
    display: &OsStr,
    deadline: Duration,
    frame_limit: usize,
    origin: Instant,
) -> ToolResult<OutputCapture> {
    let started_elapsed_ms = origin.elapsed().as_millis();
    let mut child = Command::new(tool)
        .args(["outputs", "--json"])
        .env("WAYLAND_DISPLAY", display)
        // The output command is a Wayland client, not an IPC query. Make that
        // separation explicit even if a future CLI starts inspecting this env.
        .env_remove("UMBRIEL_SOCKET")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let result = timeout(deadline, async {
        let (stdout, stderr) = tokio::try_join!(
            read_bounded(stdout, frame_limit),
            read_bounded(stderr, 64 * 1024),
        )?;
        let status = child.wait().await?;
        Ok::<_, io::Error>((status, stdout, stderr))
    })
    .await;
    let (status, stdout, stderr) = match result {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            // Retain the primary read/size/wait error after attempting cleanup.
            let _ = child.kill().await;
            return Err(error.into());
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err("output inspection timed out".into());
        }
    };
    let tool_stderr = String::from_utf8_lossy(&stderr).into_owned();
    if !status.success() {
        return Err(format!("output inspection exited {status}: {tool_stderr}").into());
    }
    // Preserve every monitor field, while rejecting incompatible/non-JSON output.
    let heads: Vec<Value> = codec::decode(&stdout)?;
    if heads.iter().any(|head| !head.is_object()) {
        return Err("output inspection must return an array of output objects".into());
    }
    Ok(OutputCapture {
        provider: "umbriel outputs --json via wlr-output-management-unstable-v1",
        tool: tool.to_string_lossy().into_owned(),
        wayland_display: display.to_string_lossy().into_owned(),
        runtime_dir: std::env::var_os("XDG_RUNTIME_DIR")
            .map(|path| path.to_string_lossy().into_owned()),
        same_compositor_verified: false,
        started_elapsed_ms,
        received_elapsed_ms: origin.elapsed().as_millis(),
        status: "ok",
        data: sonic_rs::to_value(&heads)?,
        tool_stderr,
    })
}

async fn read_bounded(reader: impl AsyncRead + Unpin, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .await?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "output inspection exceeded its byte limit",
        ));
    }
    Ok(bytes)
}
