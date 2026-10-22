//! Lógica do anansi: arquivos do workspace, variáveis, HTTP, cURL e git.
//!
//! Este crate não depende do Tauri (Seção 3 da spec). O crate do app só
//! converte os comandos do frontend em chamadas daqui, o que deixa toda a
//! lógica testável com `cargo test -p anansi-core`.

pub mod curl;
pub mod env;
pub mod error;
pub mod events;
pub mod git;
pub mod http;
pub mod model;
pub mod watcher;
pub mod workspace;

pub use error::{AppError, AppResult};
pub use workspace::Workspace;
