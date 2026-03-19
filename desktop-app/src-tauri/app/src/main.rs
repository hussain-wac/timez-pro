#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use single_instance::SingleInstance;
use std::io::Write;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

#[cfg(windows)]
use std::net::TcpStream;

#[cfg(unix)]
const INSTANCE_SOCKET_PATH: &str = "/tmp/timez-app.sock";

#[cfg(windows)]
const INSTANCE_PORT: u16 = 23400;

fn main() {
    // Set up panic handler for better crash diagnostics
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        eprintln!("[PANIC] at {location}: {message}");

        // On Windows, also try to write to a log file in temp directory
        #[cfg(windows)]
        {
            let log_path = std::env::temp_dir().join("timez-pro-crash.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                use std::io::Write as _;
                let timestamp = chrono::Utc::now().to_rfc3339();
                let _ = writeln!(file, "[{timestamp}] PANIC at {location}: {message}");
            }
        }
    }));

    // Suppress GTK warnings on Linux
    #[cfg(target_os = "linux")]
    {
        // SAFETY: This is safe at program startup before any threads are spawned
        unsafe {
            std::env::set_var("GTK_WARNINGS", "0");
            std::env::set_var("LIBAPPINDICATOR_SILENCE_WARNS", "1");
        }
    }

    let instance = match SingleInstance::new("com.wac.timez-clone") {
        Ok(instance) => instance,
        Err(err) => {
            eprintln!("[single-instance] Failed to create lock: {err}");
            return;
        }
    };

    if !instance.is_single() {
        // Try to signal the existing instance to show its window
        #[cfg(unix)]
        {
            if let Ok(mut stream) = UnixStream::connect(INSTANCE_SOCKET_PATH) {
                let _ = stream.write_all(b"show\n");
                let _ = stream.flush();
            }
        }

        #[cfg(windows)]
        {
            let addr = format!("127.0.0.1:{INSTANCE_PORT}");
            if let Ok(mut stream) = TcpStream::connect(&addr) {
                let _ = stream.write_all(b"show\n");
                let _ = stream.flush();
            }
        }

        eprintln!("[single-instance] App already running; requested existing window.");
        return;
    }

    timez_pro_lib::run();
}
