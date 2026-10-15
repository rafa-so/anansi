//! Ambientes, segredos e `.gitignore` (RF-10 a RF-14).

use crate::{prefs, state::AppState};
use anansi_core::env::{self, gitignore, EnvView};
use anansi_core::AppResult;
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::{AppHandle, State};

#[derive(Serialize)]
pub struct GitignoreStatus {
    ok: bool,
}

#[tauri::command]
pub async fn env_list(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    Ok(env::list(state.workspace()?.root()))
}

/// Variáveis com valores e só os nomes dos segredos.
#[tauri::command]
pub async fn env_get(state: State<'_, AppState>, name: String) -> AppResult<EnvView> {
    env::view(state.workspace()?.root(), &name)
}

#[tauri::command]
pub async fn env_create(state: State<'_, AppState>, name: String) -> AppResult<()> {
    env::create(state.workspace()?.root(), &name)
}

#[tauri::command]
pub async fn env_delete(app: AppHandle, state: State<'_, AppState>, name: String) -> AppResult<()> {
    let ws = state.workspace()?;
    env::delete(ws.root(), &name)?;
    let key = ws.root().display().to_string();
    if prefs::active_environment(&app, &key).as_deref() == Some(name.as_str()) {
        prefs::set_active_environment(&app, &key, None);
    }
    Ok(())
}

#[tauri::command]
pub async fn env_save_variables(
    state: State<'_, AppState>,
    name: String,
    variables: BTreeMap<String, String>,
) -> AppResult<()> {
    env::save_variables(state.workspace()?.root(), &name, variables)
}

/// `null` mantém o valor atual; chaves ausentes são removidas.
#[tauri::command]
pub async fn env_save_secrets(
    state: State<'_, AppState>,
    name: String,
    secrets: BTreeMap<String, Option<String>>,
) -> AppResult<()> {
    env::save_secrets(state.workspace()?.root(), &name, secrets)
}

#[tauri::command]
pub async fn env_reveal_secret(
    state: State<'_, AppState>,
    name: String,
    key: String,
) -> AppResult<String> {
    env::reveal_secret(state.workspace()?.root(), &name, &key)
}

/// Ambiente ativo do workspace aberto (`null` se nenhum ou se não existe mais).
#[tauri::command]
pub async fn env_get_active(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<String>> {
    let ws = state.workspace()?;
    let active = prefs::active_environment(&app, &ws.root().display().to_string());
    Ok(active.filter(|name| env::list(ws.root()).contains(name)))
}

#[tauri::command]
pub async fn env_set_active(
    app: AppHandle,
    state: State<'_, AppState>,
    name: Option<String>,
) -> AppResult<()> {
    let ws = state.workspace()?;
    prefs::set_active_environment(&app, &ws.root().display().to_string(), name.as_deref());
    Ok(())
}

/// Nomes conhecidos (variáveis + segredos), para destacar os não resolvidos.
#[tauri::command]
pub async fn env_variable_names(
    state: State<'_, AppState>,
    name: String,
) -> AppResult<Vec<String>> {
    Ok(env::load_environment(state.workspace()?.root(), &name)?.names())
}

#[tauri::command]
pub async fn gitignore_status(state: State<'_, AppState>) -> AppResult<GitignoreStatus> {
    Ok(GitignoreStatus {
        ok: gitignore::status(state.workspace()?.root()),
    })
}

#[tauri::command]
pub async fn gitignore_fix(state: State<'_, AppState>) -> AppResult<()> {
    gitignore::fix(state.workspace()?.root())
}
