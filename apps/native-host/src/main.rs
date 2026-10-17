//! Chrome/Firefox native messaging adapter: framed JSON stdin/stdout to the
//! existing single-user Watcher IPC. This process is launched by the browser.
//! Never print logs on stdout, which is reserved for native-messaging frames.
use std::io::{self, BufRead, BufReader, Read, Write};
use watcher_ipc::{PROTOCOL_VERSION, Request, Response, paths};

const MAX_FRAME_BYTES: usize = 256 * 1024;

fn read_frame(reader: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut prefix = [0u8; 4];
    match reader.read_exact(&mut prefix) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let length = u32::from_ne_bytes(prefix) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "native frame size invalid",
        ));
    }
    let mut frame = vec![0u8; length];
    reader.read_exact(&mut frame)?;
    Ok(Some(frame))
}

fn write_frame(writer: &mut impl Write, value: &Response) -> io::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "response too large",
        ));
    }
    writer.write_all(&(bytes.len() as u32).to_ne_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

fn forward(dev: bool, request: &Request) -> Result<Response, String> {
    let location = paths(dev)?.socket;
    let json = serde_json::to_string(request).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::{os::unix::net::UnixStream, time::Duration};
        let mut socket =
            UnixStream::connect(location).map_err(|_| "daemon_unavailable".to_string())?;
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        writeln!(socket, "{json}").map_err(|e| e.to_string())?;
        let mut line = String::new();
        BufReader::new(socket)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.len() > MAX_FRAME_BYTES {
            return Err("daemon response too large".into());
        }
        serde_json::from_str(&line).map_err(|e| e.to_string())
    }
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        let mut pipe = OpenOptions::new()
            .read(true)
            .write(true)
            .open(location)
            .map_err(|_| "daemon_unavailable".to_string())?;
        writeln!(pipe, "{json}").map_err(|e| e.to_string())?;
        let mut line = String::new();
        BufReader::new(pipe)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if line.len() > MAX_FRAME_BYTES {
            return Err("daemon response too large".into());
        }
        serde_json::from_str(&line).map_err(|e| e.to_string())
    }
}

fn handle(dev: bool, bytes: &[u8]) -> Response {
    let error = |message: String| Response::Error {
        version: PROTOCOL_VERSION,
        message,
    };
    let Ok(request) = serde_json::from_slice::<Request>(bytes) else {
        return error("invalid request".into());
    };
    // Native host is a deliberately restricted observation/reporting adapter:
    // no pause/resume/shutdown and no arbitrary file or command access.
    if !matches!(
        request,
        Request::Status { .. }
            | Request::Report { .. }
            | Request::BrowserBatch { .. }
            | Request::BrowserReport { .. }
    ) {
        return error("operation unavailable through browser bridge".into());
    }
    if request.version() != PROTOCOL_VERSION {
        return error("protocol mismatch".into());
    }
    forward(dev, &request).unwrap_or_else(error)
}

#[cfg(windows)]
fn enable_binary_stdio() -> io::Result<()> {
    // Windows CRT defaults stdio to text mode, which corrupts the 4-byte
    // native-messaging framing when bytes contain CR/LF (Chrome requirement).
    #[link(name = "msvcrt")]
    unsafe extern "C" {
        fn _setmode(fd: i32, mode: i32) -> i32;
    }
    const O_BINARY: i32 = 0x8000;
    for descriptor in [0, 1] {
        if unsafe { _setmode(descriptor, O_BINARY) } == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    #[cfg(windows)]
    enable_binary_stdio()?;
    let dev = std::env::args().any(|arg| arg == "--dev");
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    while let Some(bytes) = read_frame(&mut reader)? {
        write_frame(&mut writer, &handle(dev, &bytes))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_frame_roundtrips() {
        let mut output = Vec::new();
        write_frame(
            &mut output,
            &Response::Error {
                version: 1,
                message: "offline".into(),
            },
        )
        .unwrap();
        let decoded: Response =
            serde_json::from_slice(&read_frame(&mut output.as_slice()).unwrap().unwrap()).unwrap();
        assert!(matches!(decoded, Response::Error { message, .. } if message == "offline"));
    }

    #[test]
    fn oversized_frame_is_rejected_before_allocating() {
        let header = (MAX_FRAME_BYTES as u32 + 1).to_ne_bytes();
        assert!(read_frame(&mut header.as_slice()).is_err());
    }

    #[test]
    fn browser_cannot_issue_shutdown() {
        let request = serde_json::to_vec(&Request::Shutdown { version: 1 }).unwrap();
        assert!(
            matches!(handle(true, &request), Response::Error { message, .. } if message.contains("unavailable"))
        );
    }
}
