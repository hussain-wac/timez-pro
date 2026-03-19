//! Cross-platform authentication token storage.
//!
//! Stores the auth token in a platform-appropriate temporary location:
//! - Unix: `/tmp/timez-auth-store.json`
//! - Windows: `%TEMP%\timez-auth-store.json`

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Returns the platform-appropriate path for the auth store file.
///
/// On Unix, uses `/tmp/timez-auth-store.json`.
/// On Windows, uses `%TEMP%\timez-auth-store.json`.
fn get_auth_store_path() -> &'static PathBuf {
    static AUTH_STORE_PATH: OnceLock<PathBuf> = OnceLock::new();
    AUTH_STORE_PATH.get_or_init(|| {
        #[cfg(unix)]
        {
            PathBuf::from("/tmp/timez-auth-store.json")
        }
        #[cfg(windows)]
        {
            std::env::temp_dir().join("timez-auth-store.json")
        }
        #[cfg(not(any(unix, windows)))]
        {
            PathBuf::from("timez-auth-store.json")
        }
    })
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AuthStore {
    access_token: Option<String>,
}

/// Reads the stored authentication token.
///
/// Returns `None` if no token is stored or if reading fails.
#[must_use]
pub fn read_token() -> Option<String> {
    let path = get_auth_store_path();
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str::<AuthStore>(&contents)
        .ok()
        .and_then(|store| store.access_token)
}

/// Writes the authentication token to persistent storage.
///
/// # Errors
///
/// Returns an error if serialization or file writing fails.
pub fn write_token(token: Option<String>) -> Result<(), String> {
    let store = AuthStore {
        access_token: token,
    };
    let payload = serde_json::to_string(&store).map_err(|err| err.to_string())?;
    fs::write(get_auth_store_path(), payload).map_err(|err| err.to_string())
}
