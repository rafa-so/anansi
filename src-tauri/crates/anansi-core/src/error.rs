//! Erro único do backend (Seção 9 da spec).
//!
//! Serializa como `{ "code": "...", "message": "..." }`, com a mensagem já em
//! português e pronta para exibir no frontend.

use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppError {
    #[error("{0}")]
    WorkspaceNotOpen(String),
    #[error("{0}")]
    NotAWorkspace(String),
    #[error("{0}")]
    PathOutsideWorkspace(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    AlreadyExists(String),
    #[error("{0}")]
    InvalidName(String),
    #[error("{0}")]
    DuplicatedVariable(String),
    #[error("{0}")]
    InvalidFile(String),
    #[error("{0}")]
    UnsupportedSchemaVersion(String),
    #[error("{0}")]
    GitignoreNotConfigured(String),
    #[error("{0}")]
    InvalidUrl(String),
    #[error("{0}")]
    HttpError(String),
    #[error("{0}")]
    ResponseTooLarge(String),
    #[error("{0}")]
    Cancelled(String),
    #[error("{0}")]
    IoError(String),
}

impl AppError {
    /// Código estável do erro, igual ao campo `code` serializado.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::WorkspaceNotOpen(_) => "WORKSPACE_NOT_OPEN",
            AppError::NotAWorkspace(_) => "NOT_A_WORKSPACE",
            AppError::PathOutsideWorkspace(_) => "PATH_OUTSIDE_WORKSPACE",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::AlreadyExists(_) => "ALREADY_EXISTS",
            AppError::InvalidName(_) => "INVALID_NAME",
            AppError::DuplicatedVariable(_) => "DUPLICATED_VARIABLE",
            AppError::InvalidFile(_) => "INVALID_FILE",
            AppError::UnsupportedSchemaVersion(_) => "UNSUPPORTED_SCHEMA_VERSION",
            AppError::GitignoreNotConfigured(_) => "GITIGNORE_NOT_CONFIGURED",
            AppError::InvalidUrl(_) => "INVALID_URL",
            AppError::HttpError(_) => "HTTP_ERROR",
            AppError::ResponseTooLarge(_) => "RESPONSE_TOO_LARGE",
            AppError::Cancelled(_) => "CANCELLED",
            AppError::IoError(_) => "IO_ERROR",
        }
    }

    pub fn workspace_not_open() -> Self {
        AppError::WorkspaceNotOpen("Nenhum workspace aberto.".into())
    }

    pub fn not_found(rel: &str) -> Self {
        AppError::NotFound(format!("Item não encontrado: {rel}"))
    }

    /// Converte um erro de E/S, com o caminho envolvido na mensagem.
    pub fn io(err: &std::io::Error, path: &Path) -> Self {
        if err.kind() == std::io::ErrorKind::NotFound {
            return AppError::NotFound(format!("Item não encontrado: {}", path.display()));
        }
        AppError::IoError(format!(
            "Falha de leitura ou escrita em {}: {err}",
            path.display()
        ))
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_code_and_message() {
        let err = AppError::InvalidName("Nome vazio.".into());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "INVALID_NAME", "message": "Nome vazio." })
        );
    }

    #[test]
    fn code_matches_serialized_code_for_every_variant() {
        let all = [
            AppError::WorkspaceNotOpen(String::new()),
            AppError::NotAWorkspace(String::new()),
            AppError::PathOutsideWorkspace(String::new()),
            AppError::NotFound(String::new()),
            AppError::AlreadyExists(String::new()),
            AppError::InvalidName(String::new()),
            AppError::DuplicatedVariable(String::new()),
            AppError::InvalidFile(String::new()),
            AppError::UnsupportedSchemaVersion(String::new()),
            AppError::GitignoreNotConfigured(String::new()),
            AppError::InvalidUrl(String::new()),
            AppError::HttpError(String::new()),
            AppError::ResponseTooLarge(String::new()),
            AppError::Cancelled(String::new()),
            AppError::IoError(String::new()),
        ];
        for err in all {
            let json = serde_json::to_value(&err).unwrap();
            assert_eq!(json["code"], err.code());
        }
    }

    #[test]
    fn io_not_found_becomes_not_found() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "x");
        assert_eq!(AppError::io(&io, Path::new("a.json")).code(), "NOT_FOUND");
    }
}
