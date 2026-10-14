//! Typed command shell over `Endpoint` one-shot execution.
//!
//! Every command opens a fresh connection, validates locally before any bytes
//! are transmitted, and prints native reply data decoded through the checked
//! protocol models. Remote rejections, local decode failures, and transport or
//! deadline failures stay distinct in the output; transmission uncertainty is
//! never hidden. Mutating commands (`overview-toggle`, `workspace-switch`,
//! `raw`) execute only when this example is run deliberately against a real
//! socket; the tests below use scripted Unix peers and never touch a live
//! compositor.
//!
//! Typed command words are tokenized with POSIX shell quoting (`shlex`), so
//! workspace names and outputs may contain spaces or non-ASCII text:
//! `workspace-switch --name "my space" --output 'DP 1'`. The `raw` escape
//! hatch deliberately keeps the remainder of its line verbatim, because
//! quoting there belongs to the compositor action grammar, not this REPL.
//!
//! Pass an explicit socket path argument to reach a scripted peer; without one,
//! discovery follows `UMBRIEL_SOCKET` / `XDG_RUNTIME_DIR` + `WAYLAND_DISPLAY`.
use std::{
    error::Error,
    fmt,
    io::{self, BufRead, Write},
    path::PathBuf,
};

use shlex::Shlex;

use umbriel_client::{Discovery, Endpoint, Error as WireError, Options, RawReply};
use umbriel_protocol::{
    action::{Action, ValidationError, WorkspaceSelector, WorkspaceTarget},
    model::{
        Color, KeyboardLayouts, Layers, Submap, Tearing, Window, Windows, Workspace, Workspaces,
    },
    request::{Query, Request},
};

/// Explicit workspace selection parsed from the command line: an index is a
/// cursor-relative number, a name is compositor-disambiguated literal text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Selection {
    Index(usize),
    Name(String),
}

/// One parsed REPL line. Parsing and validation are purely local.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    Quit,
    Wire(WireCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WireCommand {
    Query(Query),
    OverviewToggle,
    WorkspaceSwitch {
        selection: Selection,
        output: Option<String>,
    },
    /// Escape hatch: deliberately bypasses local action validation. JSON
    /// escaping and the request-size limit still apply through the encoder.
    Raw(String),
}

/// Local syntax and validation failures; nothing was transmitted.
/// `Eq` is unavailable: `ValidationError` carries rejected `f64` arguments.
#[derive(Debug, Clone, PartialEq)]
enum UsageError {
    Unknown(String),
    UnexpectedArgument(String),
    UnknownFlag(String),
    MissingFlagValue(&'static str),
    InvalidFlagValue { flag: &'static str, value: String },
    DuplicateFlag(&'static str),
    MissingSelector,
    ConflictingSelectors,
    EmptyRawAction,
    Validation(ValidationError),
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(word) => write!(f, "unknown command {word:?}; try 'help'"),
            Self::UnexpectedArgument(argument) => {
                write!(f, "unexpected argument {argument:?}")
            }
            Self::UnknownFlag(flag) => write!(f, "unknown flag {flag:?}"),
            Self::MissingFlagValue(flag) => write!(f, "flag {flag} requires a value"),
            Self::InvalidFlagValue { flag, value } => {
                write!(f, "flag {flag} received invalid value {value:?}")
            }
            Self::DuplicateFlag(flag) => write!(f, "flag {flag} was given more than once"),
            Self::MissingSelector => {
                write!(
                    f,
                    "workspace-switch requires exactly one of --index N or --name NAME"
                )
            }
            Self::ConflictingSelectors => write!(f, "--index and --name are mutually exclusive"),
            Self::EmptyRawAction => write!(f, "raw requires action text after it"),
            Self::Validation(error) => write!(f, "action rejected locally: {error}"),
        }
    }
}

/// Parse one REPL line. `Ok(None)` is an empty line, not an error. Typed
/// command words use POSIX shell quoting; only `raw` reads its remainder
/// verbatim from the original line.
fn parse_line(line: &str) -> Result<Option<Command>, UsageError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let mut words = Shlex::new(trimmed);
    let Some(head) = words.next() else {
        return Ok(None);
    };
    let command = match head.as_str() {
        "help" => {
            reject_arguments(&mut words)?;
            Command::Help
        }
        "quit" | "exit" => {
            reject_arguments(&mut words)?;
            Command::Quit
        }
        "overview-toggle" => {
            reject_arguments(&mut words)?;
            Command::Wire(WireCommand::OverviewToggle)
        }
        "raw" => {
            // The raw action text is the remainder of the line verbatim, with
            // quoting left to the compositor action grammar.
            let rest = trimmed
                .split_once(char::is_whitespace)
                .map_or("", |(_, rest)| rest)
                .trim_start();
            if rest.is_empty() {
                return Err(UsageError::EmptyRawAction);
            }
            Command::Wire(WireCommand::Raw(rest.to_owned()))
        }
        "workspace-switch" => Command::Wire(parse_workspace_switch(&mut words)?),
        name => {
            let Some(query) = Query::ALL.iter().copied().find(|q| q.as_str() == name) else {
                return Err(UsageError::Unknown(name.to_owned()));
            };
            reject_arguments(&mut words)?;
            Command::Wire(WireCommand::Query(query))
        }
    };
    Ok(Some(command))
}

fn reject_arguments(words: &mut Shlex<'_>) -> Result<(), UsageError> {
    match words.next() {
        None => Ok(()),
        Some(argument) => Err(UsageError::UnexpectedArgument(argument)),
    }
}

fn parse_workspace_switch(words: &mut Shlex<'_>) -> Result<WireCommand, UsageError> {
    let mut index: Option<usize> = None;
    let mut name: Option<String> = None;
    let mut output: Option<String> = None;
    while let Some(flag) = words.next() {
        match flag.as_str() {
            "--index" => {
                let Some(value) = words.next() else {
                    return Err(UsageError::MissingFlagValue("--index"));
                };
                if index.is_some() || name.is_some() {
                    return Err(UsageError::ConflictingSelectors);
                }
                index = Some(value.parse().map_err(|_| UsageError::InvalidFlagValue {
                    flag: "--index",
                    value,
                })?);
            }
            "--name" => {
                let Some(value) = words.next() else {
                    return Err(UsageError::MissingFlagValue("--name"));
                };
                if index.is_some() || name.is_some() {
                    return Err(UsageError::ConflictingSelectors);
                }
                name = Some(value);
            }
            "--output" => {
                let Some(value) = words.next() else {
                    return Err(UsageError::MissingFlagValue("--output"));
                };
                if output.is_some() {
                    return Err(UsageError::DuplicateFlag("--output"));
                }
                output = Some(value);
            }
            other => return Err(UsageError::UnknownFlag(other.to_owned())),
        }
    }
    let selection = match (index, name) {
        (Some(index), None) => Selection::Index(index),
        (None, Some(name)) => Selection::Name(name),
        (None, None) => return Err(UsageError::MissingSelector),
        (Some(_), Some(_)) => return Err(UsageError::ConflictingSelectors),
    };
    Ok(WireCommand::WorkspaceSwitch { selection, output })
}

/// Build the typed switch action from an explicit selection. Validation
/// failures are local and happen before any connection exists.
fn workspace_switch_action(
    selection: &Selection,
    output: &Option<String>,
) -> Result<Action<'static>, UsageError> {
    let selector = match selection {
        Selection::Index(index) => WorkspaceSelector::index(*index),
        Selection::Name(name) => WorkspaceSelector::name(name),
    }
    .map_err(UsageError::Validation)?;
    let mut target = WorkspaceTarget::new(selector);
    if let Some(output) = output {
        target = target.on_output(output).map_err(UsageError::Validation)?;
    }
    Ok(Action::workspace_switch(target))
}

/// Native reply data decoded through the checked models, ready to print.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Report {
    Query {
        name: &'static str,
        headline: String,
        rows: Vec<String>,
    },
    Action {
        text: String,
        bypassed: bool,
    },
}

#[derive(Debug)]
enum ExecError {
    /// Rejected before any socket existed: parsing or action validation.
    Local(UsageError),
    /// Remote rejection, local decode failure, or transport/deadline failure.
    Wire(WireError),
}

impl From<WireError> for ExecError {
    fn from(error: WireError) -> Self {
        Self::Wire(error)
    }
}

/// Execute one wire command through a fresh one-shot connection.
async fn execute(
    endpoint: &Endpoint,
    options: &Options,
    command: &WireCommand,
) -> Result<Report, ExecError> {
    match command {
        WireCommand::Query(query) => {
            let reply = endpoint.request(Request::Query(*query), options).await?;
            typed_report(*query, &reply).map_err(ExecError::Wire)
        }
        WireCommand::OverviewToggle => {
            let action = Action::overview_toggle();
            let reply = endpoint.request(action.request(), options).await?;
            accepted(&action, &reply, false)
        }
        WireCommand::WorkspaceSwitch { selection, output } => {
            let action = workspace_switch_action(selection, output).map_err(ExecError::Local)?;
            let reply = endpoint.request(action.request(), options).await?;
            accepted(&action, &reply, false)
        }
        WireCommand::Raw(text) => {
            let action = Action::raw(text.clone());
            let reply = endpoint.request(action.request(), options).await?;
            accepted(&action, &reply, true)
        }
    }
}

fn accepted(action: &Action<'_>, reply: &RawReply, bypassed: bool) -> Result<Report, ExecError> {
    reply
        .decode()
        .map_err(ExecError::Wire)
        .map(|()| Report::Action {
            text: action.as_str().to_owned(),
            bypassed,
        })
}

/// Decode one query reply into its typed model. A decode failure means the
/// compositor replied with data outside the checked model, which stays a local
/// error distinct from any remote rejection.
fn typed_report(query: Query, reply: &RawReply) -> Result<Report, WireError> {
    let (name, headline, rows) = match query {
        Query::Windows => {
            let windows: Windows = reply.decode()?;
            let headline = format!("{} window(s)", windows.len());
            let rows = windows.iter().map(window_row).collect();
            ("windows", headline, rows)
        }
        Query::Workspaces => {
            let workspaces: Workspaces = reply.decode()?;
            let headline = format!("{} workspace(s)", workspaces.len());
            let rows = workspaces.iter().map(workspace_row).collect();
            ("workspaces", headline, rows)
        }
        Query::Submap => {
            let submap: Submap = reply.decode()?;
            let headline = match submap {
                Some(name) => format!("active submap: {name}"),
                None => "active submap: default context (none active)".to_owned(),
            };
            ("submap", headline, Vec::new())
        }
        Query::Layers => {
            let layers: Layers = reply.decode()?;
            let headline = format!("{} layer surface(s)", layers.len());
            let rows = layers
                .iter()
                .map(|surface| {
                    let output = if surface.output.is_empty() {
                        "(none)"
                    } else {
                        surface.output.as_str()
                    };
                    format!(
                        "{}\t{}\toutput={output}\tmapped={}",
                        surface.layer.as_str(),
                        surface.namespace,
                        surface.mapped
                    )
                })
                .collect();
            ("layers", headline, rows)
        }
        Query::Color => {
            let color: Color = reply.decode()?;
            let headline = color_headline(&color);
            let rows = color
                .outputs
                .iter()
                .map(|output| {
                    format!(
                        "output {}: hdr={} (requested {}, active {}), format {}, transfer {}, primaries {}, sdr white {}",
                        output.name,
                        output.hdr_mode.as_str(),
                        output.hdr_requested,
                        output.hdr_active,
                        output.render_format,
                        output.transfer_function.as_str(),
                        output.primaries.as_str(),
                        output.sdr_white
                    )
                })
                .chain(color.surfaces.iter().map(|surface| {
                    format!(
                        "surface {} ({} / {}): transfer {}, primaries {}, max cll {}, max fall {}",
                        surface.id,
                        surface.app_id,
                        surface.title,
                        surface.transfer_function.as_str(),
                        surface.primaries.as_str(),
                        surface.max_cll,
                        surface.max_fall
                    )
                }))
                .collect();
            ("color", headline, rows)
        }
        Query::Tearing => {
            let tearing: Tearing = reply.decode()?;
            let headline = format!(
                "tearing protocol supported: {}; {} output(s), {} surface(s)",
                tearing.protocol,
                tearing.outputs.len(),
                tearing.surfaces.len()
            );
            let rows = tearing
                .outputs
                .iter()
                .map(|output| {
                    let presentation = match (
                        output.last_presentation_flags,
                        output.last_presentation_presented,
                        output.last_presentation_vsync,
                    ) {
                        (Some(flags), Some(presented), Some(vsync)) => {
                            format!(" last presentation: flags={flags:#x} presented={presented} vsync={vsync}")
                        }
                        _ => String::new(),
                    };
                    format!(
                        "output {}: allowed={} requested={} last commit tearing={}{}",
                        output.name,
                        output.allowed,
                        output.requested,
                        output.last_commit_tearing,
                        presentation
                    )
                })
                .chain(tearing.surfaces.iter().map(|surface| {
                    let rule = match surface.rule_override {
                        Some(true) => "forced",
                        Some(false) => "denied",
                        None => "policy",
                    };
                    format!(
                        "surface {} ({} / {}): hint={} fullscreen={} eligible={} override={rule}",
                        surface.id,
                        surface.app_id,
                        surface.title,
                        surface.hint.as_str(),
                        surface.fullscreen,
                        surface.eligible
                    )
                }))
                .collect();
            ("tearing", headline, rows)
        }
        Query::KeyboardLayouts => {
            let layouts: KeyboardLayouts = reply.decode()?;
            let current = layouts
                .names
                .get(layouts.current_index as usize)
                .map(String::as_str);
            let headline = match current {
                Some(current) => {
                    format!("{} layout(s); current: {current}", layouts.names.len())
                }
                None => format!(
                    "{} layout(s); current index {} is out of range",
                    layouts.names.len(),
                    layouts.current_index
                ),
            };
            let rows = layouts
                .names
                .iter()
                .enumerate()
                .map(|(position, name)| {
                    let marker = if position as u32 == layouts.current_index {
                        "*"
                    } else {
                        " "
                    };
                    format!("{marker} {name}")
                })
                .collect();
            ("keyboard-layouts", headline, rows)
        }
    };
    Ok(Report::Query {
        name,
        headline,
        rows,
    })
}

fn color_headline(color: &Color) -> String {
    format!(
        "color manager supported: {}; renderer: input transform {}, output transform {}, timeline {}; {} output(s), {} surface(s)",
        color.color_manager,
        color.renderer.input_color_transform,
        color.renderer.output_color_transform,
        color.renderer.timeline,
        color.outputs.len(),
        color.surfaces.len()
    )
}

fn window_row(window: &Window) -> String {
    let storage = if window.scratchpad.is_empty() {
        window.workspace.to_string()
    } else {
        format!("scratchpad:{}", window.scratchpad)
    };
    let flags = marked(
        "active",
        window.active,
        &[
            ("focused", window.focused),
            ("urgent", window.urgent),
            ("floating", window.floating),
            ("xwayland", window.xwayland),
        ],
    );
    format!(
        "{}\t{}\t{}\tws={}\t[{}]\t{}x{}+{}+{}\tpid={}",
        window.id,
        window.app_id,
        window.title,
        storage,
        flags,
        window.w,
        window.h,
        window.x,
        window.y,
        window.pid
    )
}

fn workspace_row(workspace: &Workspace) -> String {
    let name = if workspace.name.is_empty() {
        "(unnamed)"
    } else {
        workspace.name.as_str()
    };
    let flags = marked(
        "active",
        workspace.active,
        &[
            ("focused", workspace.focused),
            ("occupied", workspace.occupied),
        ],
    );
    format!(
        "{}\t{}\t{}\toutput={}\t[{}]\tlayout={}",
        workspace.index,
        workspace.id,
        name,
        workspace.output,
        flags,
        workspace.layout.as_str()
    )
}

/// The primary flag is always shown (`active` or `!active`); secondary flags
/// appear only when set, keeping rows readable without hiding the seat state.
fn marked(primary: &str, primary_on: bool, rest: &[(&str, bool)]) -> String {
    let mut flags = vec![if primary_on {
        primary.to_owned()
    } else {
        format!("!{primary}")
    }];
    flags.extend(
        rest.iter()
            .filter(|(_, on)| *on)
            .map(|(name, _)| (*name).to_owned()),
    );
    flags.join(",")
}

fn print_help() {
    println!(
        "grammar: typed command words use POSIX shell quoting ('single', \"double\", backslash);"
    );
    println!("         raw takes the remainder of its line verbatim");
    println!("queries: windows, workspaces, submap, layers, color, tearing, keyboard-layouts");
    println!("actions: overview-toggle");
    println!("         workspace-switch (--index N | --name NAME) [--output NAME]");
    println!(
        "         NAME may contain spaces or non-ASCII text: --name \"my space\" --output 'DP 1'"
    );
    println!(
        "escape:  raw <action text>   (verbatim rest of line; bypasses local action validation)"
    );
    println!("other:   help, quit");
}

fn print_report(report: Report) {
    match report {
        Report::Query {
            name,
            headline,
            rows,
        } => {
            println!("{name}: {headline}");
            for row in rows {
                println!("  {row}");
            }
        }
        Report::Action { text, bypassed } => {
            if bypassed {
                println!("ok (raw action, local validation deliberately bypassed): {text}");
            } else {
                println!("ok (validated action): {text}");
            }
            println!("  native ok accepts execution; it is not rendering convergence");
        }
    }
}

/// Print failures with remote versus local evidence preserved. Deadline and
/// transmission-uncertainty cases keep their exact uncertainty wording.
fn print_failure(error: ExecError) {
    match error {
        ExecError::Local(usage) => println!("local usage error (nothing was transmitted): {usage}"),
        ExecError::Wire(WireError::Remote(reason)) => println!("remote rejection: {reason}"),
        ExecError::Wire(WireError::Decode(source)) => {
            println!("local failure: native reply is outside the checked model: {source}")
        }
        ExecError::Wire(other) => println!("local failure (connection or timing): {other}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let discovery = match args.next() {
        // Explicit path reaches a scripted peer without mutating the environment.
        Some(path) => Discovery {
            explicit: Some(PathBuf::from(path)),
            ..Discovery::from_env()
        },
        None => Discovery::from_env(),
    };
    let endpoint = discovery.resolve()?;
    let options = Options::default();
    println!("umbriel typed shell at {}", endpoint.path().display());
    print_help();
    let stdin = io::stdin();
    // Blocking stdin read: the only async work is one serially issued request.
    let mut lines = stdin.lock().lines();
    loop {
        print!("umbriel> ");
        io::stdout().flush()?;
        let Some(line) = lines.next() else {
            break;
        };
        let line = line?;
        match parse_line(&line) {
            Ok(None) => {}
            Ok(Some(Command::Help)) => print_help(),
            Ok(Some(Command::Quit)) => break,
            Ok(Some(Command::Wire(command))) => {
                match execute(&endpoint, &options, &command).await {
                    Ok(report) => print_report(report),
                    Err(error) => print_failure(error),
                }
            }
            Err(usage) => println!("local usage error (nothing was transmitted): {usage}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::UnixListener,
    };

    /// Parse and execute one REPL line against a scripted Unix peer that
    /// asserts the exact request line and replies with `reply` before clean
    /// EOF. No live compositor is involved; mutating commands are answered by
    /// the scripted peer only.
    async fn scripted_line(
        line: &str,
        expected_request: &str,
        reply: Vec<u8>,
    ) -> Result<Report, ExecError> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scripted.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let expected = format!("{expected_request}\n");
        let peer = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut request = String::new();
            stream.read_line(&mut request).await.unwrap();
            assert_eq!(request, expected, "unexpected one-shot request");
            stream.get_mut().write_all(&reply).await.unwrap();
        });
        let command = parse_line(line).unwrap().unwrap();
        let Command::Wire(command) = command else {
            panic!("expected a wire command from {line:?}");
        };
        let result = execute(&Endpoint::new(path), &Options::default(), &command).await;
        peer.await.unwrap();
        result
    }

    fn wire(line: &str) -> WireCommand {
        match parse_line(line).unwrap().unwrap() {
            Command::Wire(command) => command,
            other => panic!("expected a wire command from {line:?}: {other:?}"),
        }
    }

    fn window_json(id: &str, app_id: &str, title: &str, active: bool) -> String {
        format!(
            r#"{{"id":"{id}","workspace":"out:1","scratchpad":"","active":{active},"app_id":"{app_id}","title":"{title}","xdg_tag":"","content_type":"none","floating":false,"focused":false,"urgent":false,"xwayland":false,"pid":-1,"x":0,"y":0,"w":10,"h":10}}"#
        )
    }

    #[test]
    fn parses_all_seven_queries_and_rejects_extra_arguments() {
        for (name, query) in Query::ALL.iter().map(|query| (query.as_str(), *query)) {
            assert_eq!(
                wire(name),
                WireCommand::Query(query),
                "query name {name} should parse"
            );
            assert!(matches!(
                parse_line(&format!("{name} extra")),
                Err(UsageError::UnexpectedArgument(_))
            ));
        }
        assert!(matches!(
            parse_line("nope"),
            Err(UsageError::Unknown(word)) if word == "nope"
        ));
        assert_eq!(parse_line("   ").unwrap(), None);
        assert_eq!(parse_line("help").unwrap(), Some(Command::Help));
        assert_eq!(parse_line("quit").unwrap(), Some(Command::Quit));
        assert_eq!(parse_line("exit").unwrap(), Some(Command::Quit));
    }

    #[test]
    fn workspace_switch_requires_exactly_one_explicit_selector() {
        assert_eq!(
            wire("workspace-switch --index 3"),
            WireCommand::WorkspaceSwitch {
                selection: Selection::Index(3),
                output: None
            }
        );
        assert_eq!(
            wire("workspace-switch --name web --output DP-1"),
            WireCommand::WorkspaceSwitch {
                selection: Selection::Name("web".to_owned()),
                output: Some("DP-1".to_owned())
            }
        );
        for (line, expected) in [
            ("workspace-switch", UsageError::MissingSelector),
            (
                "workspace-switch --index 3 --name web",
                UsageError::ConflictingSelectors,
            ),
            (
                "workspace-switch --index",
                UsageError::MissingFlagValue("--index"),
            ),
            (
                "workspace-switch --index three",
                UsageError::InvalidFlagValue {
                    flag: "--index",
                    value: "three".to_owned(),
                },
            ),
            (
                "workspace-switch --index 2 --top DP-1",
                UsageError::UnknownFlag("--top".to_owned()),
            ),
            (
                "workspace-switch --index 2 --output a --output b",
                UsageError::DuplicateFlag("--output"),
            ),
            ("raw", UsageError::EmptyRawAction),
        ] {
            assert_eq!(parse_line(line).unwrap_err(), expected, "line: {line}");
        }
    }

    #[test]
    fn quoted_words_express_spaced_and_non_ascii_names() {
        assert_eq!(
            wire(r#"workspace-switch --name "my space" --output 'DP 1'"#),
            WireCommand::WorkspaceSwitch {
                selection: Selection::Name("my space".to_owned()),
                output: Some("DP 1".to_owned())
            }
        );
        assert_eq!(
            wire(r"workspace-switch --name my\ space"),
            WireCommand::WorkspaceSwitch {
                selection: Selection::Name("my space".to_owned()),
                output: None
            }
        );
        assert_eq!(
            wire(r#"workspace-switch --name 'веб'"#),
            WireCommand::WorkspaceSwitch {
                selection: Selection::Name("веб".to_owned()),
                output: None
            }
        );
        // Quoting belongs to this REPL; tokens reach the typed builder unquoted.
        assert_eq!(parse_line(r#""help""#).unwrap(), Some(Command::Help));
    }

    #[tokio::test]
    async fn typed_queries_decode_native_data_and_preserve_remote_and_local_errors() {
        let report = scripted_line(
            "submap",
            r#"{"cmd":"submap"}"#,
            b"{\"ok\":\"resize\"}\n".to_vec(),
        )
        .await
        .unwrap();
        match report {
            Report::Query {
                name,
                headline,
                rows,
            } => {
                assert_eq!(name, "submap");
                assert_eq!(headline, "active submap: resize");
                assert!(rows.is_empty());
            }
            other => panic!("unexpected report: {other:?}"),
        }

        let reply = format!(
            "{{\"ok\":[{},{}]}}\n",
            window_json("w1", "terminal", "logs", true),
            window_json("w2", "browser", "docs", false),
        );
        let report = scripted_line("windows", r#"{"cmd":"windows"}"#, reply.into_bytes())
            .await
            .unwrap();
        match report {
            Report::Query {
                name,
                headline,
                rows,
            } => {
                assert_eq!(name, "windows");
                assert_eq!(headline, "2 window(s)");
                assert!(
                    rows[0].contains("w1")
                        && rows[0].contains("terminal")
                        && rows[0].contains("active")
                );
                assert!(rows[1].contains("w2") && rows[1].contains("!active"));
            }
            other => panic!("unexpected report: {other:?}"),
        }

        let error = scripted_line(
            "submap",
            r#"{"cmd":"submap"}"#,
            b"{\"err\":\"unknown submap state\"}\n".to_vec(),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error, ExecError::Wire(WireError::Remote(ref reason)) if reason == "unknown submap state"),
            "remote rejection must stay remote: {error:?}"
        );

        let error = scripted_line("submap", r#"{"cmd":"submap"}"#, b"{\"ok\":42}\n".to_vec())
            .await
            .unwrap_err();
        assert!(
            matches!(error, ExecError::Wire(WireError::Decode(_))),
            "a native reply outside the model is a local decode failure: {error:?}"
        );
    }

    #[tokio::test]
    async fn workspace_switch_index_name_and_output_encoding() {
        for (line, request, action_text) in [
            (
                "workspace-switch --index 3",
                r#"{"cmd":"msg","arg":"workspace-switch:3"}"#,
                "workspace-switch:3",
            ),
            (
                "workspace-switch --name 3",
                r#"{"cmd":"msg","arg":"workspace-switch:\"3\""}"#,
                "workspace-switch:\"3\"",
            ),
            (
                "workspace-switch --name web",
                r#"{"cmd":"msg","arg":"workspace-switch:web"}"#,
                "workspace-switch:web",
            ),
            (
                "workspace-switch --index 2 --output DP-1",
                r#"{"cmd":"msg","arg":"workspace-switch:2/DP-1"}"#,
                "workspace-switch:2/DP-1",
            ),
            (
                r#"workspace-switch --name "my space""#,
                r#"{"cmd":"msg","arg":"workspace-switch:my space"}"#,
                "workspace-switch:my space",
            ),
            (
                r#"workspace-switch --index 1 --output "DP 1""#,
                r#"{"cmd":"msg","arg":"workspace-switch:1/DP 1"}"#,
                "workspace-switch:1/DP 1",
            ),
            (
                "workspace-switch --name веб",
                "{\"cmd\":\"msg\",\"arg\":\"workspace-switch:веб\"}",
                "workspace-switch:веб",
            ),
            (
                "overview-toggle",
                r#"{"cmd":"msg","arg":"overview-toggle"}"#,
                "overview-toggle",
            ),
        ] {
            let report = scripted_line(line, request, b"{\"ok\":null}\n".to_vec())
                .await
                .unwrap_or_else(|error| panic!("{line}: {error:?}"));
            match report {
                Report::Action {
                    text,
                    bypassed: false,
                } => assert_eq!(text, action_text),
                other => panic!("unexpected report for {line}: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn raw_action_bypasses_validation_but_not_the_wire_encoder() {
        let report = scripted_line(
            r#"raw future-action:do "quoted" things"#,
            r#"{"cmd":"msg","arg":"future-action:do \"quoted\" things"}"#,
            b"{\"ok\":null}\n".to_vec(),
        )
        .await
        .unwrap();
        match report {
            Report::Action {
                text,
                bypassed: true,
            } => assert_eq!(text, "future-action:do \"quoted\" things"),
            other => panic!("unexpected report: {other:?}"),
        }
    }

    #[tokio::test]
    async fn invalid_selections_fail_locally_without_connecting() {
        // The socket is bound but never accepted; any connection attempt would
        // block until the request deadline rather than returning promptly.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("idle.sock");
        let _listener = UnixListener::bind(&path).unwrap();
        for line in [
            "workspace-switch --index 0",
            "workspace-switch --index 65",
            // Single quotes keep the double quotes literal in the token, so the
            // typed builder rejects the unrepresentable leading quote.
            r#"workspace-switch --name '"quoted"'"#,
            "workspace-switch --name a/b",
            "workspace-switch --index 1 --output bad/output",
        ] {
            let command = wire(line);
            let error = execute(&Endpoint::new(&path), &Options::default(), &command)
                .await
                .unwrap_err();
            assert!(
                matches!(error, ExecError::Local(UsageError::Validation(_))),
                "{line} must fail validation without a connection: {error:?}"
            );
        }
    }
}
