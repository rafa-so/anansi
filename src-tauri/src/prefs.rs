//! Preferências locais (RNF-07, Seção 10 da spec), gravadas com o `tauri-plugin-store`.
//!
//! Em build de debug, `ANANSI_E2E_PREFS_DIR` troca a pasta das preferências
//! e `ANANSI_E2E_WORKSPACE` abre um workspace no início, sem diálogo
//! (testes E2E, Seção 15.4).

use anansi_core::http::HttpDefaults;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;

pub const LAST_WORKSPACE: &str = "lastWorkspace";
pub const ACTIVE_ENVIRONMENT: &str = "activeEnvironment";
pub const DEFAULT_VERIFY_TLS: &str = "defaultVerifyTls";
pub const DEFAULT_TIMEOUT_MS: &str = "defaultTimeoutMs";

fn store_path() -> PathBuf {
    if cfg!(debug_assertions) {
        if let Ok(dir) = std::env::var("ANANSI_E2E_PREFS_DIR") {
            return PathBuf::from(dir).join("prefs.json");
        }
    }
    PathBuf::from("prefs.json")
}

/// Identifier das versões até a 0.3.1, que gravavam as preferências em outra pasta.
const LEGACY_IDENTIFIER: &str = "io.github.your-username.anansi";

/// Copia o `prefs.json` da pasta do identifier antigo, se a pasta atual ainda
/// não tiver um. Roda no `setup`, antes de qualquer leitura do store.
pub fn migrate_legacy<R: Runtime>(app: &AppHandle<R>) {
    if store_path().is_absolute() {
        return;
    }
    let Ok(dir) = app.path().app_data_dir() else {
        return;
    };
    let target = dir.join(store_path());
    let Some(legacy) = dir
        .parent()
        .map(|p| p.join(LEGACY_IDENTIFIER).join(store_path()))
    else {
        return;
    };
    if target.exists() || !legacy.exists() {
        return;
    }
    let copied = std::fs::create_dir_all(&dir).and_then(|_| std::fs::copy(&legacy, &target));
    if let Err(e) = copied {
        eprintln!(
            "anansi: não foi possível migrar as preferências de {}: {e}",
            legacy.display()
        );
    }
}

/// Workspace a abrir no início em testes E2E (só em debug).
pub fn e2e_workspace() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("ANANSI_E2E_WORKSPACE")
        .ok()
        .map(PathBuf::from)
}

pub fn get<R: Runtime>(app: &AppHandle<R>, key: &str) -> Option<Value> {
    app.store(store_path()).ok()?.get(key)
}

pub fn set<R: Runtime>(app: &AppHandle<R>, key: &str, value: Value) {
    if let Ok(store) = app.store(store_path()) {
        store.set(key, value);
        let _ = store.save();
    }
}

/// Preferências globais editáveis na interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalPrefs {
    pub default_verify_tls: bool,
    pub default_timeout_ms: u64,
}

pub fn global<R: Runtime>(app: &AppHandle<R>) -> GlobalPrefs {
    let defaults = HttpDefaults::default();
    GlobalPrefs {
        default_verify_tls: get(app, DEFAULT_VERIFY_TLS)
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.verify_tls),
        default_timeout_ms: get(app, DEFAULT_TIMEOUT_MS)
            .and_then(|v| v.as_u64())
            .filter(|v| *v > 0)
            .unwrap_or(defaults.timeout_ms),
    }
}

pub fn http_defaults<R: Runtime>(app: &AppHandle<R>) -> HttpDefaults {
    let prefs = global(app);
    HttpDefaults {
        verify_tls: prefs.default_verify_tls,
        timeout_ms: prefs.default_timeout_ms,
        follow_redirects: true,
    }
}

/// Ambiente ativo de um workspace (`activeEnvironment` é um mapa por caminho).
pub fn active_environment<R: Runtime>(app: &AppHandle<R>, workspace: &str) -> Option<String> {
    get(app, ACTIVE_ENVIRONMENT)?
        .get(workspace)?
        .as_str()
        .map(str::to_string)
}

pub fn set_active_environment<R: Runtime>(app: &AppHandle<R>, workspace: &str, name: Option<&str>) {
    let mut map = get(app, ACTIVE_ENVIRONMENT)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    map.insert(
        workspace.to_string(),
        name.map(Value::from).unwrap_or(Value::Null),
    );
    set(app, ACTIVE_ENVIRONMENT, Value::Object(map));
}
