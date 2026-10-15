//! cURL como formato de troca (Seção 11 da spec): exportar (bash e PowerShell 7) e importar (bash).

pub mod export;
pub mod import;

pub use export::{export, CurlFormat, Exported};
pub use import::{import, Imported};
