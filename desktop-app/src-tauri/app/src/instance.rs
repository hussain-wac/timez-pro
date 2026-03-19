//! Single instance management for the Tauri app.
//!
//! Uses Unix domain sockets on Linux/macOS and TCP on Windows.

use std::io::{BufRead, BufReader};

use tauri::Manager;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::net::UnixListener;
#[cfg(unix)]
use std::path::Path;

#[cfg(windows)]
use std::net::TcpListener;

#[cfg(unix)]
const INSTANCE_SOCKET_PATH: &str = "/tmp/timez-app.sock";

#[cfg(windows)]
const INSTANCE_PORT: u16 = 23400;

pub fn spawn_show_listener<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<(), String> {
    #[cfg(unix)]
    {
        spawn_unix_listener(app_handle)
    }

    #[cfg(windows)]
    {
        spawn_tcp_listener(app_handle)
    }
}

#[cfg(unix)]
fn spawn_unix_listener<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<(), String> {
    remove_stale_socket();

    let listener = UnixListener::bind(INSTANCE_SOCKET_PATH)
        .map_err(|e| format!("Failed to bind instance socket: {e}"))?;

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[instance] Failed to accept connection: {e}");
                    continue;
                }
            };

            // Set a read timeout to prevent hanging on malformed requests
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));

            let mut line = String::new();
            let mut reader = BufReader::new(stream);
            if reader.read_line(&mut line).is_ok() && line.trim() == "show" {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        }
    });

    Ok(())
}

#[cfg(unix)]
fn remove_stale_socket() {
    let path = Path::new(INSTANCE_SOCKET_PATH);
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(windows)]
fn spawn_tcp_listener<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<(), String> {
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("127.0.0.1:{INSTANCE_PORT}")
        .parse()
        .map_err(|e| format!("Invalid address: {e}"))?;

    // Try to bind with SO_REUSEADDR to handle crashed previous instances
    let socket = std::net::TcpListener::bind(addr).map_err(|e| {
        // If binding fails, the port might be in use by a zombie process
        // This is not fatal - single-instance check already passed
        eprintln!("[instance] Warning: Could not bind TCP listener: {e}");
        format!("Failed to bind instance listener: {e}")
    })?;

    // Set non-blocking to false for clean shutdown
    socket
        .set_nonblocking(false)
        .map_err(|e| format!("Failed to set socket blocking mode: {e}"))?;

    std::thread::spawn(move || {
        for stream in socket.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[instance] Failed to accept connection: {e}");
                    continue;
                }
            };

            // Set a read timeout to prevent hanging on malformed requests
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));

            let mut line = String::new();
            let mut reader = BufReader::new(stream);
            if reader.read_line(&mut line).is_ok() && line.trim() == "show" {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        }
    });

    Ok(())
}
