//! cURL: copiar (bash e PowerShell 7) e colar (RF-15, RF-16).

use crate::state::AppState;
use anansi_core::curl::{self, CurlFormat, Exported, Imported};
use anansi_core::env;
use anansi_core::model::RequestFile;
use anansi_core::AppResult;
use tauri::State;

#[tauri::command]
pub async fn curl_export(
    state: State<'_, AppState>,
    request: RequestFile,
    env: Option<String>,
    resolve: bool,
    format: CurlFormat,
) -> AppResult<Exported> {
    let data = match (&env, resolve) {
        (Some(name), true) => Some(env::load_environment(state.workspace()?.root(), name)?),
        _ => None,
    };
    Ok(curl::export(&request, data.as_ref(), resolve, format))
}

#[tauri::command]
pub async fn curl_import(text: String) -> AppResult<Imported> {
    curl::import(&text)
}
