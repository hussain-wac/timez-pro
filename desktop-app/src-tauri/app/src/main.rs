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
    // Suppress GTK warnings on Linux
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("GTK_WARNINGS", "0");
        std::env::set_var("LIBAPPINDICATOR_SILENCE_WARNS", "1");
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
            let addr = format!("127.0.0.1:{}", INSTANCE_PORT);
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
