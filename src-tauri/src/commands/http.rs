//! Execução HTTP (RF-08, RF-20, RNF-10).

use crate::{prefs, state::AppState};
use anansi_core::env;
use anansi_core::http::HttpResponse;
use anansi_core::model::RequestFile;
use anansi_core::AppResult;
use std::path::Path;
use tauri::{AppHandle, State};

/// Executa o rascunho da aba (inclusive alterações não salvas). As variáveis e os
/// segredos são lidos do disco a cada envio, para refletir edições recentes.
#[tauri::command]
pub async fn http_send(
    app: AppHandle,
    state: State<'_, AppState>,
    exec_id: String,
    request: RequestFile,
    env: Option<String>,
) -> AppResult<HttpResponse> {
    let ws = state.workspace()?;
    let data = match env {
        Some(name) => Some(env::load_environment(ws.root(), &name)?),
        None => None,
    };
    let executor = state.executor.clone();
    executor
        .send(
            &exec_id,
            &request,
            data.as_ref(),
            prefs::http_defaults(&app),
        )
        .await
}

#[tauri::command]
pub async fn http_cancel(state: State<'_, AppState>, exec_id: String) -> AppResult<()> {
    state.executor.cancel(&exec_id);
    Ok(())
}

/// Grava o body completo da resposta no caminho escolhido no diálogo "Salvar".
#[tauri::command]
pub async fn http_save_body(
    state: State<'_, AppState>,
    exec_id: String,
    dest_path: String,
) -> AppResult<()> {
    state.executor.save_body(&exec_id, Path::new(&dest_path))
}

/// Libera o body guardado (aba fechada).
#[tauri::command]
pub async fn http_release(state: State<'_, AppState>, exec_id: String) -> AppResult<()> {
    state.executor.release(&exec_id);
    Ok(())
}
