//! Cross-platform service runtime.
//!
//! Uses Unix domain sockets on Linux/macOS and TCP on Windows.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::time::Duration;

use timez_core::protocol::{Request, RequestEnvelope, ResponseData, ResponseEnvelope};

use crate::REQUEST_TOKEN;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(unix)]
use std::path::Path;

#[cfg(windows)]
use std::net::{TcpListener, TcpStream};

// Type aliases for cross-platform compatibility
#[cfg(unix)]
type IpcStream = UnixStream;

#[cfg(windows)]
type IpcStream = TcpStream;

pub fn parse_parent_pid() -> Option<u32> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--parent-pid" {
            return args.next().and_then(|value| value.parse::<u32>().ok());
        }
    }
    None
}

#[cfg(unix)]
pub fn run_server<F>(socket_path: PathBuf, parent_pid: Option<u32>, handler: F) -> Result<(), String>
where
    F: Fn(Request) -> Result<ResponseData, String> + Send + Sync + 'static,
{
    remove_stale_socket(&socket_path);
    let listener = UnixListener::bind(&socket_path)
        .map_err(|e| format!("Failed to bind {}: {e}", socket_path.display()))?;

    spawn_parent_watchdog(parent_pid, Some(socket_path.clone()), None);

    for stream in listener.incoming() {
        let shutdown = match stream {
            Ok(stream) => handle_stream(stream, &handler),
            Err(e) => {
                eprintln!("[service] Failed to accept connection: {e}");
                false
            }
        };

        if shutdown {
            break;
        }
    }

    remove_stale_socket(&socket_path);
    Ok(())
}

#[cfg(windows)]
pub fn run_server<F>(port: u16, parent_pid: Option<u32>, handler: F) -> Result<(), String>
where
    F: Fn(Request) -> Result<ResponseData, String> + Send + Sync + 'static,
{
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|e| format!("Invalid address: {e}"))?;

    // Try binding with retries in case of port conflicts from crashed services
    let listener = {
        let mut last_error = None;
        let mut attempts = 0;
        loop {
            match TcpListener::bind(addr) {
                Ok(l) => break l,
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);
                    if attempts >= 3 {
                        return Err(format!(
                            "Failed to bind {} after {} attempts: {}",
                            addr,
                            attempts,
                            last_error.unwrap()
                        ));
                    }
                    // Wait briefly before retry
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    };

    eprintln!("[service] Listening on {addr}");
    spawn_parent_watchdog(parent_pid, None, Some(port));

    for stream in listener.incoming() {
        let shutdown = match stream {
            Ok(stream) => {
                // Set read timeout to prevent hanging on malformed requests
                let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
                handle_stream(stream, &handler)
            }
            Err(e) => {
                eprintln!("[service] Failed to accept connection: {e}");
                false
            }
        };

        if shutdown {
            break;
        }
    }

    Ok(())
}

#[cfg(unix)]
pub fn send_request(
    socket_path: &Path,
    request: Request,
) -> Result<ResponseData, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|e| e.to_string())?;
    send_request_on_stream(&mut stream, request)
}

#[cfg(windows)]
pub fn send_request(
    port: u16,
    request: Request,
) -> Result<ResponseData, String> {
    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect(&addr).map_err(|e| e.to_string())?;
    send_request_on_stream(&mut stream, request)
}

fn send_request_on_stream(stream: &mut (impl Write + Read), request: Request) -> Result<ResponseData, String> {
    let envelope = RequestEnvelope {
        token: REQUEST_TOKEN.to_string(),
        request,
    };
    let payload = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
    stream.write_all(payload.as_bytes()).map_err(|e: std::io::Error| e.to_string())?;
    stream.write_all(b"\n").map_err(|e: std::io::Error| e.to_string())?;
    stream.flush().map_err(|e: std::io::Error| e.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e: std::io::Error| e.to_string())?;

    let response: ResponseEnvelope =
        serde_json::from_str(&line).map_err(|e| format!("Invalid response: {e}"))?;

    if !response.ok {
        return Err(response.error.unwrap_or_else(|| "Unknown service error".to_string()));
    }

    response
        .data
        .ok_or_else(|| "Missing response payload".to_string())
}

fn handle_stream<F>(stream: IpcStream, handler: &F) -> bool
where
    F: Fn(Request) -> Result<ResponseData, String>,
{
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if let Err(e) = reader.read_line(&mut line) {
        eprintln!("[service] Failed to read request: {e}");
        return false;
    }

    let envelope = match serde_json::from_str::<RequestEnvelope>(&line) {
        Ok(envelope) => envelope,
        Err(e) => {
            let _ = write_response(
                reader.get_mut(),
                ResponseEnvelope {
                    ok: false,
                    data: None,
                    error: Some(format!("Invalid request: {e}")),
                },
            );
            return false;
        }
    };

    if envelope.token != REQUEST_TOKEN {
        let _ = write_response(
            reader.get_mut(),
            ResponseEnvelope {
                ok: false,
                data: None,
                error: Some("Unauthorized request".to_string()),
            },
        );
        return false;
    }

    let request = envelope.request;
    let shutdown = matches!(request, Request::Shutdown);
    let response = match handler(request) {
        Ok(data) => ResponseEnvelope {
            ok: true,
            data: Some(data),
            error: None,
        },
        Err(error) => ResponseEnvelope {
            ok: false,
            data: None,
            error: Some(error),
        },
    };

    let _ = write_response(reader.get_mut(), response);
    shutdown
}

fn write_response(stream: &mut IpcStream, response: ResponseEnvelope) -> Result<(), String> {
    let payload = serde_json::to_string(&response).map_err(|e| e.to_string())?;
    stream.write_all(payload.as_bytes()).map_err(|e: std::io::Error| e.to_string())?;
    stream.write_all(b"\n").map_err(|e: std::io::Error| e.to_string())?;
    stream.flush().map_err(|e: std::io::Error| e.to_string())
}

#[cfg(unix)]
fn spawn_parent_watchdog(parent_pid: Option<u32>, socket_path: Option<PathBuf>, _port: Option<u16>) {
    let Some(parent_pid) = parent_pid else {
        return;
    };

    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));
        if !std::path::Path::new(&format!("/proc/{parent_pid}")).exists() {
            if let Some(ref path) = socket_path {
                remove_stale_socket(path);
            }
            std::process::exit(0);
        }
    });
}

#[cfg(windows)]
fn spawn_parent_watchdog(parent_pid: Option<u32>, _socket_path: Option<PathBuf>, _port: Option<u16>) {
    let Some(parent_pid) = parent_pid else {
        return;
    };

    std::thread::spawn(move || {
        use std::ptr::null_mut;

        // Windows API constants
        const SYNCHRONIZE: u32 = 0x00100000;
        const WAIT_OBJECT_0: u32 = 0;

        #[link(name = "kernel32")]
        extern "system" {
            fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
            fn WaitForSingleObject(hHandle: *mut std::ffi::c_void, dwMilliseconds: u32) -> u32;
            fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
        }

        unsafe {
            let handle = OpenProcess(SYNCHRONIZE, 0, parent_pid);
            if handle.is_null() {
                // Can't open parent process, assume it's gone
                std::process::exit(0);
            }

            // Wait indefinitely for parent process to exit
            let result = WaitForSingleObject(handle, 0xFFFFFFFF);
            CloseHandle(handle);

            if result == WAIT_OBJECT_0 {
                std::process::exit(0);
            }
        }
    });
}

#[cfg(unix)]
fn remove_stale_socket(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}
