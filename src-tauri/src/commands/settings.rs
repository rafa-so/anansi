//! Preferências globais (Seção 10).

use crate::prefs::{self, GlobalPrefs};
use anansi_core::{AppError, AppResult};
use serde_json::Value;
use tauri::AppHandle;

#[tauri::command]
pub async fn prefs_get(app: AppHandle) -> AppResult<GlobalPrefs> {
    Ok(prefs::global(&app))
}

#[tauri::command]
pub async fn prefs_set(app: AppHandle, prefs: GlobalPrefs) -> AppResult<()> {
    if prefs.default_timeout_ms == 0 {
        return Err(AppError::InvalidName(
            "O timeout precisa ser maior que zero.".into(),
        ));
    }
    prefs::set(
        &app,
        prefs::DEFAULT_VERIFY_TLS,
        Value::from(prefs.default_verify_tls),
    );
    prefs::set(
        &app,
        prefs::DEFAULT_TIMEOUT_MS,
        Value::from(prefs.default_timeout_ms),
    );
    Ok(())
}
