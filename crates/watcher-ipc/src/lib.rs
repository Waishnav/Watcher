//! Versioned JSON-lines protocol for a per-user local connection.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use watcher_core::{Interval, Report, Snapshot};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug)]
pub struct Paths {
    pub data: PathBuf,
    pub socket: PathBuf,
}

pub fn paths(dev: bool) -> Result<Paths, String> {
    let override_dir = std::env::var_os("WATCHER_DEV_HOME").map(PathBuf::from);
    let data = if let Some(root) = override_dir.as_ref() {
        root.clone()
    } else if dev {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(".watcher-dev")
    } else {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .ok_or("missing HOME or XDG_DATA_HOME")?;
        base.join("watcher")
    };
    let socket = if override_dir.is_some() || dev {
        data.join("daemon.sock")
    } else {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| data.clone());
        runtime.join("watcher/daemon.sock")
    };
    Ok(Paths { data, socket })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Status {
        version: u32,
    },
    Report {
        version: u32,
        from_ms: i64,
        to_ms: i64,
    },
    Timeline {
        version: u32,
        from_ms: i64,
        to_ms: i64,
        limit: usize,
    },
    Pause {
        version: u32,
    },
    Resume {
        version: u32,
    },
    Shutdown {
        version: u32,
    },
}

impl Request {
    pub fn version(&self) -> u32 {
        match self {
            Self::Status { version }
            | Self::Report { version, .. }
            | Self::Timeline { version, .. }
            | Self::Pause { version }
            | Self::Resume { version }
            | Self::Shutdown { version } => *version,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    Status {
        version: u32,
        snapshot: Snapshot,
        collector: String,
        detail: String,
    },
    Report {
        version: u32,
        report: Report,
    },
    Timeline {
        version: u32,
        intervals: Vec<Interval>,
    },
    Ok {
        version: u32,
        message: String,
    },
    Error {
        version: u32,
        message: String,
    },
}
