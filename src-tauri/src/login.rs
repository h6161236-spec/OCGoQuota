use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};
use tokio::time::{sleep, Instant};
use url::Url;
use uuid::Uuid;

use crate::{commands::AppState, opencode};

const MAIN_WINDOW_LABEL: &str = "main";
const LOGIN_BASE: &str = "https://auth.opencode.ai/authorize";
const LOGIN_REDIRECT_URI: &str = "https://opencode.ai/auth/callback";
const COOKIE_URL: &str = "https://opencode.ai";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const COOKIE_POLL_INTERVAL: Duration = Duration::from_millis(500);
const NAVIGATION_SETTLE: Duration = Duration::from_millis(800);

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum OpenCodeLoginResult {
    Ok {
        workspace_id: String,
        auth_cookie: String,
    },
    Cancelled,
    Error {
        error: String,
    },
}

struct ActiveLoginGuard(Arc<AtomicBool>);

impl Drop for ActiveLoginGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn login_url() -> Result<Url, String> {
    let mut url = Url::parse(LOGIN_BASE).map_err(|error| error.to_string())?;
    url.query_pairs_mut()
        .append_pair("client_id", "app")
        .append_pair("redirect_uri", LOGIN_REDIRECT_URI)
        .append_pair("response_type", "code")
        .append_pair("state", &Uuid::new_v4().to_string());
    Ok(url)
}

fn workspace_from_url(url: &Url) -> Option<String> {
    url.path_segments()?
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|segments| {
            (segments[0] == "workspace" && segments[1].starts_with("wrk_"))
                .then(|| segments[1].to_string())
        })
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn store_result(app: &AppHandle, result: OpenCodeLoginResult) {
    if let Ok(mut slot) = app.state::<AppState>().login_result.lock() {
        *slot = Some(result);
    }
}

fn return_to_app(window: &WebviewWindow, return_url: Url) {
    let _ = window.navigate(return_url);
}

#[tauri::command]
pub fn start_opencode_login(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let active = Arc::clone(&state.login_active);
    if active
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("an OpenCode login is already in progress".into());
    }

    let window = match app.get_webview_window(MAIN_WINDOW_LABEL) {
        Some(window) => window,
        None => {
            active.store(false, Ordering::Release);
            return Err("the main application window is unavailable".into());
        }
    };
    let return_url = window.url().map_err(|error| {
        active.store(false, Ordering::Release);
        error.to_string()
    })?;
    let login_url = login_url().map_err(|error| {
        active.store(false, Ordering::Release);
        error
    })?;
    let cookie_url = Url::parse(COOKIE_URL).map_err(|error| {
        active.store(false, Ordering::Release);
        error.to_string()
    })?;
    if let Ok(mut slot) = state.login_result.lock() {
        *slot = None;
    }
    drop(state);

    tauri::async_runtime::spawn(async move {
        let _guard = ActiveLoginGuard(active);
        sleep(Duration::from_millis(100)).await;
        if let Err(error) = window.navigate(login_url) {
            store_result(
                &app,
                OpenCodeLoginResult::Error {
                    error: format!("failed to load OpenCode login: {error}"),
                },
            );
            return;
        }

        sleep(NAVIGATION_SETTLE).await;
        let deadline = Instant::now() + LOGIN_TIMEOUT;
        let mut url_workspace = None;
        let mut left_app = false;
        while Instant::now() < deadline {
            if app.get_webview_window(MAIN_WINDOW_LABEL).is_none() {
                return;
            }

            let current_url = window.url().ok();
            let mut returned_to_app = false;
            if let Some(current_url) = current_url.as_ref() {
                if same_origin(current_url, &return_url) {
                    returned_to_app = left_app;
                } else {
                    left_app = true;
                }
                if let Some(workspace_id) = workspace_from_url(current_url) {
                    url_workspace = Some(workspace_id);
                }
            }

            if let Ok(cookies) = window.cookies_for_url(cookie_url.clone()) {
                if let Some(auth) = cookies
                    .iter()
                    .find(|cookie| cookie.name() == "auth" && !cookie.value().is_empty())
                {
                    let auth_cookie = format!("auth={}", auth.value());
                    let workspace_id = match url_workspace {
                        Some(workspace_id) => workspace_id,
                        None => match opencode::resolve_workspace_id("Default", &auth_cookie).await
                        {
                            Ok(workspace_id) => workspace_id,
                            Err(error) => {
                                store_result(
                                    &app,
                                    OpenCodeLoginResult::Error {
                                        error: error.to_string(),
                                    },
                                );
                                return_to_app(&window, return_url);
                                return;
                            }
                        },
                    };
                    store_result(
                        &app,
                        OpenCodeLoginResult::Ok {
                            workspace_id,
                            auth_cookie,
                        },
                    );
                    return_to_app(&window, return_url);
                    return;
                }
            }

            if returned_to_app {
                store_result(&app, OpenCodeLoginResult::Cancelled);
                return;
            }

            sleep(COOKIE_POLL_INTERVAL).await;
        }

        store_result(
            &app,
            OpenCodeLoginResult::Error {
                error: "login timed out; please try again".into(),
            },
        );
        return_to_app(&window, return_url);
    });

    Ok(())
}

#[tauri::command]
pub fn take_opencode_login_result(app: AppHandle) -> Result<Option<OpenCodeLoginResult>, String> {
    app.state::<AppState>()
        .login_result
        .lock()
        .map_err(|_| "OpenCode login state is unavailable".to_string())
        .map(|mut slot| slot.take())
}

#[cfg(test)]
mod tests {
    use super::{same_origin, workspace_from_url};

    #[test]
    fn extracts_workspace_id_from_usage_url() {
        let url = "https://opencode.ai/workspace/wrk_abc123/usage"
            .parse()
            .unwrap();
        assert_eq!(workspace_from_url(&url).as_deref(), Some("wrk_abc123"));
    }

    #[test]
    fn ignores_urls_without_workspace_id() {
        let url = "https://opencode.ai/auth/callback".parse().unwrap();
        assert_eq!(workspace_from_url(&url), None);
    }

    #[test]
    fn compares_origins_without_path_or_query() {
        let app = "http://tauri.localhost/settings".parse().unwrap();
        let returned = "http://tauri.localhost/settings?login=cancelled"
            .parse()
            .unwrap();
        let remote = "https://opencode.ai/workspace/wrk_123".parse().unwrap();
        assert!(same_origin(&app, &returned));
        assert!(!same_origin(&app, &remote));
    }
}
