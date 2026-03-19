use std::sync::mpsc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::api::{self, AuthToken};
use crate::local_store::LocalTimeStorage;
use crate::models::{AuthResponse, AuthUser};
use crate::timer_state::TimerState;

pub fn google_login(
    google_id_token: String,
    auth: State<'_, AuthToken>,
    _timer: State<'_, TimerState>,
    local_store: State<'_, LocalTimeStorage>,
) -> Result<AuthResponse, String> {
    let response = api::google_login(&google_id_token)?;

    // Store the access token
    {
        let mut auth_state = auth.inner().lock().map_err(|e| e.to_string())?;
        auth_state.access_token = Some(response.access_token.clone());
    }

    // Persist token for crash recovery
    local_store.set_auth_token(response.access_token.clone());

    Ok(response)
}

/// Start Google OAuth - runs in background thread to avoid blocking UI
pub fn start_google_auth(
    client_id: String,
    client_secret: String,
    app_handle: AppHandle,
    auth: State<'_, AuthToken>,
    _timer: State<'_, TimerState>,
    local_store: State<'_, LocalTimeStorage>,
) -> Result<String, String> {
    // Spawn background thread
    let app = app_handle.clone();

    std::thread::spawn(move || {
        let result = api::google_oauth_via_browser(&client_id, &client_secret);

        match result {
            Ok(response) => {
                // Store token using app_handle
                {
                    let auth_state = app.state::<AuthToken>();
                    if let Ok(mut state) = auth_state.inner().lock() {
                        state.access_token = Some(response.access_token.clone());
                    }
                }

                // Persist token
                {
                    let local_store = app.state::<LocalTimeStorage>();
                    local_store.set_auth_token(response.access_token.clone());
                }

                // Emit success event
                let _ = app.emit("auth-success", &response);
            }
            Err(e) => {
                let _ = app.emit("auth-error", &e);
            }
        }
    });

    Ok("OAuth started".to_string())
}

pub fn validate_token(
    token: String,
    auth: State<'_, AuthToken>,
    _timer: State<'_, TimerState>,
    local_store: State<'_, LocalTimeStorage>,
) -> Result<AuthUser, String> {
    let user = api::get_me(&token)?;

    // Restore the token in state
    {
        let mut auth_state = auth.inner().lock().map_err(|e| e.to_string())?;
        auth_state.access_token = Some(token.clone());
    }

    // Persist token for crash recovery
    local_store.set_auth_token(token.clone());

    Ok(user)
}

pub fn logout(
    auth: State<'_, AuthToken>,
    local_store: State<'_, LocalTimeStorage>,
) -> Result<(), String> {
    let mut auth_state = auth.inner().lock().map_err(|e| e.to_string())?;
    auth_state.access_token = None;
    local_store.clear_auth_token();
    Ok(())
}
