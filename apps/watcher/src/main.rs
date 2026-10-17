//! The small, colorful Watcher command-line client. All reads go via the daemon.
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use clap::{Parser, Subcommand};
use std::{io::IsTerminal, path::PathBuf};
use watcher_core::{ActivityState, Interval, Report};
use watcher_ipc::{PROTOCOL_VERSION, Request, Response, paths};

#[derive(Parser)]
#[command(name = "watcher", version, about = "Local-first screen-time tracker")]
struct Args {
    #[arg(long, global = true, help = "Use the isolated .watcher-dev profile")]
    dev: bool,
    #[arg(long, global = true, help = "Print the IPC response as JSON")]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// App usage and active time today (or on another date).
    Today {
        #[arg(long)]
        date: Option<String>,
    },
    /// Active time in the current ISO week.
    Week,
    /// Ranked applications for the current day.
    Apps,
    /// Interval timeline for a date.
    Timeline {
        #[arg(long)]
        date: Option<String>,
    },
    /// Collector and daemon state.
    Status,
    /// Diagnose missing platform capabilities.
    Doctor,
    /// Exclude time from tracking until resumed.
    Pause,
    /// Resume tracking.
    Resume,
    /// Stop the per-user daemon.
    Shutdown,
    /// Open the separate OpenTUI client.
    Tui,
}

fn day_range(date: Option<String>) -> Result<(i64, i64), String> {
    let date = match date {
        Some(raw) => NaiveDate::parse_from_str(&raw, "%Y-%m-%d").map_err(|e| e.to_string())?,
        None => Local::now().date_naive(),
    };
    let next = date.succ_opt().ok_or("date outside supported range")?;
    let first = Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .ok_or("no local midnight at start of date")?;
    let last = Local
        .from_local_datetime(&next.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .ok_or("no local midnight at end of date")?;
    Ok((first.timestamp_millis(), last.timestamp_millis()))
}

fn week_range() -> Result<(i64, i64), String> {
    let today = Local::now().date_naive();
    let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    let (from, _) = day_range(Some(monday.to_string()))?;
    Ok((from, Local::now().timestamp_millis()))
}

fn request(dev: bool, req: &Request) -> Result<Response, String> {
    #[cfg(unix)]
    {
        use std::{
            io::{BufRead, BufReader, Write},
            os::unix::net::UnixStream,
            time::Duration,
        };
        let location = paths(dev)?.socket;
        let mut stream = UnixStream::connect(&location)
            .map_err(|err| format!("cannot connect to watcherd at {}: {err}\nRun: cargo run -p watcherd -- --dev (for --dev), or start your installed daemon", location.display()))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        writeln!(
            stream,
            "{}",
            serde_json::to_string(req).map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?;
        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.is_empty() {
            return Err("daemon closed connection without responding".into());
        }
        serde_json::from_str(&line).map_err(|e| e.to_string())
    }
    #[cfg(windows)]
    {
        use std::{
            fs::OpenOptions,
            io::{BufRead, BufReader, Write},
            thread,
            time::Duration,
        };
        let address = paths(dev)?.socket;
        let mut pipe = (0..20)
            .find_map(|_| {
                let attempt = OpenOptions::new().read(true).write(true).open(&address);
                if let Ok(file) = attempt {
                    return Some(file);
                }
                thread::sleep(Duration::from_millis(100));
                None
            })
            .ok_or_else(|| {
                format!(
                    "cannot connect to watcherd named pipe: {}",
                    address.display()
                )
            })?;
        writeln!(
            pipe,
            "{}",
            serde_json::to_string(req).map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?;
        let mut line = String::new();
        BufReader::new(pipe)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.is_empty() {
            return Err("daemon closed the named pipe".into());
        }
        serde_json::from_str(&line).map_err(|e| e.to_string())
    }
}

fn pretty(ms: i64) -> String {
    let secs = ms.max(0) / 1000;
    format!(
        "{}h {:02}m {:02}s",
        secs / 3600,
        (secs / 60) % 60,
        secs % 60
    )
}

fn colored(text: &str, color: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[{color}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

fn output(response: Response, json: bool) -> Result<(), String> {
    if let Response::Error { message, .. } = &response {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?
            );
        }
        return Err(message.clone());
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    match response {
        Response::Status {
            collector,
            snapshot,
            detail,
            ..
        } => {
            println!("{}\n", colored("Watcher · Collector status", "1;34", color));
            println!("  Collector       {collector}");
            println!(
                "  App             {}",
                snapshot.app.as_deref().unwrap_or("—")
            );
            println!("  Activity        {:?}", snapshot.state);
            println!(
                "  Foreground      {}",
                if snapshot.focus_available {
                    "supported"
                } else {
                    "unavailable"
                }
            );
            println!(
                "  Input idle      {}",
                if snapshot.input_available {
                    "supported"
                } else {
                    "unavailable"
                }
            );
            println!("  Details         {detail}");
            if !snapshot.input_available || !snapshot.focus_available {
                println!(
                    "\n{}",
                    colored(
                        "Coverage incomplete: missing signals are NOT counted as active time.",
                        "33",
                        color
                    )
                );
            }
        }
        Response::Report { report, .. } => report_output(&report, color),
        Response::Timeline { intervals, .. } => timeline_output(&intervals, color),
        Response::Ok { message, .. } => println!("{}", colored(&message, "32", color)),
        Response::Error { message, .. } => return Err(message),
    }
    Ok(())
}

fn report_output(report: &Report, color: bool) {
    println!(
        "\n   {}        {}",
        colored("Active interaction time", "1;33", color),
        colored(&pretty(report.active_ms), "1;34", color)
    );
    println!(
        "   Idle: {}   Unavailable: {}   Paused: {}",
        pretty(report.idle_ms),
        pretty(report.unknown_ms),
        pretty(report.paused_ms)
    );
    println!("   ────────────────────────────────────────────────");
    println!("           {}", colored("Application usage", "1;31", color));
    println!("   ────────────────────────────────────────────────");
    for app in &report.apps {
        println!(
            "   {:<28} {:>15}",
            colored(&app.app, "32", color),
            pretty(app.duration_ms)
        );
    }
    if report.apps.is_empty() {
        println!("   No active application time recorded yet.");
    }
}

fn timeline_output(intervals: &[Interval], color: bool) {
    println!("{}", colored("Watcher · Activity timeline", "1;34", color));
    for interval in intervals {
        let local = chrono::DateTime::from_timestamp_millis(interval.start_ms)
            .map(|dt| dt.with_timezone(&Local));
        let time = local
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or("?".into());
        let state = match interval.state {
            ActivityState::Active => "active",
            ActivityState::Idle => "idle",
            ActivityState::Unknown => "unknown",
            ActivityState::Paused => "paused",
        };
        println!(
            "  {time}  {:<8} {:<20} {}",
            state,
            interval.app.as_deref().unwrap_or("—"),
            pretty(interval.end_ms - interval.start_ms)
        );
    }
    if intervals.is_empty() {
        println!("  No intervals yet.");
    }
}

fn launch_tui(dev: bool) -> Result<(), String> {
    let location = paths(dev)?.socket;
    let installed = paths(false)?.data.join("tui/src/index.ts");
    let path: PathBuf = std::env::var_os("WATCHER_TUI_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if !dev && installed.exists() {
                installed
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tui/src/index.ts")
            }
        });
    if !path.exists() {
        return Err(
            "OpenTUI source not installed; set WATCHER_TUI_SCRIPT or install the TUI package"
                .into(),
        );
    }
    let status = std::process::Command::new("bun")
        .arg("run")
        .arg(&path)
        .env("WATCHER_SOCKET", location)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("OpenTUI exited: {status}"))
    }
}

fn run(args: Args) -> Result<(), String> {
    if matches!(args.command, Command::Tui) {
        return launch_tui(args.dev);
    }
    let version = PROTOCOL_VERSION;
    let req = match args.command {
        Command::Today { date } => {
            let (from_ms, to_ms) = day_range(date)?;
            Request::Report {
                version,
                from_ms,
                to_ms,
            }
        }
        Command::Timeline { date } => {
            let (from_ms, to_ms) = day_range(date)?;
            Request::Timeline {
                version,
                from_ms,
                to_ms,
                limit: 1000,
            }
        }
        Command::Week => {
            let (from_ms, to_ms) = week_range()?;
            Request::Report {
                version,
                from_ms,
                to_ms,
            }
        }
        Command::Apps => {
            let (from_ms, to_ms) = day_range(None)?;
            Request::Report {
                version,
                from_ms,
                to_ms,
            }
        }
        Command::Status | Command::Doctor => Request::Status { version },
        Command::Pause => Request::Pause { version },
        Command::Resume => Request::Resume { version },
        Command::Shutdown => Request::Shutdown { version },
        Command::Tui => unreachable!(),
    };
    output(request(args.dev, &req)?, args.json)
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("watcher: {err}");
        std::process::exit(1);
    }
}
