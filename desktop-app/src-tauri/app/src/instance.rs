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
            let Ok(stream) = stream else {
                continue;
            };

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
    let addr = format!("127.0.0.1:{}", INSTANCE_PORT);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("Failed to bind instance listener: {e}"))?;

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };

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
