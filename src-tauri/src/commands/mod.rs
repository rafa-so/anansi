//! Handlers `#[tauri::command]`: finos, só convertem e delegam ao `anansi-core`.
//!
//! Os comandos são `async` para não rodar na thread principal da janela.

pub mod collections;
pub mod curl;
pub mod environments;
pub mod folders;
pub mod git;
pub mod http;
pub mod requests;
pub mod settings;
pub mod workspace;
