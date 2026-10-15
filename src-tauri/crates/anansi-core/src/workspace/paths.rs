//! Validação de caminhos relativos ao workspace (Seção 5 da spec).
//!
//! O frontend sempre envia caminhos relativos, com `/`. Aqui eles são
//! validados lexicalmente (sem tocar no disco) e convertidos para absolutos.

use crate::error::{AppError, AppResult};
use std::path::{Component, Path, PathBuf};

/// Comprimento máximo do caminho relativo de um item novo ou renomeado (Seção 4.4).
pub const MAX_RELATIVE_PATH_LEN: usize = 180;

const RESERVED_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Indica se o nome (com ou sem extensão) é reservado no Windows (`con`, `com1.json`...).
pub fn is_reserved_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_lowercase();
    RESERVED_NAMES.contains(&stem.as_str())
}

fn outside(rel: &str) -> AppError {
    AppError::PathOutsideWorkspace(format!("Caminho inválido ou fora do workspace: {rel}"))
}

/// Valida um caminho relativo. A string vazia representa a raiz do workspace.
pub fn validate_relative(rel: &str) -> AppResult<()> {
    if rel.is_empty() {
        return Ok(());
    }
    if rel.contains('\\') || rel.contains(':') || rel.starts_with('/') || rel.ends_with('/') {
        return Err(outside(rel));
    }
    for part in rel.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(outside(rel));
        }
        if part.ends_with('.') || part.ends_with(' ') || is_reserved_name(part) {
            return Err(outside(rel));
        }
        if part
            .chars()
            .any(|c| c.is_control() || "<>\"|?*".contains(c))
        {
            return Err(outside(rel));
        }
    }
    Ok(())
}

/// Garante que um caminho relativo novo não passe do limite de comprimento.
pub fn check_length(rel: &str) -> AppResult<()> {
    if rel.chars().count() > MAX_RELATIVE_PATH_LEN {
        return Err(AppError::InvalidName(format!(
            "Caminho muito longo ({} caracteres; o limite é {MAX_RELATIVE_PATH_LEN}). Use nomes mais curtos.",
            rel.chars().count()
        )));
    }
    Ok(())
}

/// Converte um caminho relativo validado em absoluto, dentro da raiz.
pub fn resolve(root: &Path, rel: &str) -> AppResult<PathBuf> {
    validate_relative(rel)?;
    let mut path = root.to_path_buf();
    if !rel.is_empty() {
        for part in rel.split('/') {
            path.push(part);
        }
    }
    if path.exists() {
        let canonical = dunce::canonicalize(&path).map_err(|e| AppError::io(&e, &path))?;
        if !is_inside(root, &canonical) {
            return Err(outside(rel));
        }
    }
    Ok(path)
}

/// Junta um caminho relativo pai e um nome, no formato do frontend.
pub fn join_rel(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// Caminho relativo do pai (`a/b/c.json` → `a/b`; `a` → ``).
pub fn parent_rel(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(p, _)| p).unwrap_or("")
}

/// Último componente de um caminho relativo.
pub fn file_name_rel(rel: &str) -> &str {
    rel.rsplit_once('/').map(|(_, n)| n).unwrap_or(rel)
}

/// Converte um caminho absoluto dentro da raiz para o formato relativo do frontend.
pub fn to_relative(root: &Path, abs: &Path) -> AppResult<String> {
    let rel = abs
        .strip_prefix(root)
        .map_err(|_| outside(&abs.display().to_string()))?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| match c {
            Component::Normal(s) => Ok(s.to_string_lossy().into_owned()),
            _ => Err(outside(&abs.display().to_string())),
        })
        .collect::<AppResult<_>>()?;
    Ok(parts.join("/"))
}

/// Compara componente a componente se `path` está dentro de `root`.
/// No Windows a comparação não diferencia maiúsculas (`d:\` e `D:\`).
pub fn is_inside(root: &Path, path: &Path) -> bool {
    let root_parts: Vec<_> = root.components().collect();
    let parts: Vec<_> = path.components().collect();
    if parts.len() < root_parts.len() {
        return false;
    }
    root_parts
        .iter()
        .zip(parts.iter())
        .all(|(a, b)| same_component(a.as_os_str(), b.as_os_str()))
}

fn same_component(a: &std::ffi::OsStr, b: &std::ffi::OsStr) -> bool {
    if cfg!(windows) {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("")]
    #[case("store-api")]
    #[case("store-api/products/listar-produtos.json")]
    #[case("store-api/ação.json")]
    fn accepts_valid_paths(#[case] rel: &str) {
        assert!(validate_relative(rel).is_ok(), "{rel}");
    }

    #[rstest]
    #[case("..")]
    #[case("a/../b")]
    #[case("./a")]
    #[case("/a")]
    #[case("a/")]
    #[case("a//b")]
    #[case("C:/x")]
    #[case("C:")]
    #[case("a\\b")]
    #[case("\\\\servidor\\share")]
    #[case("a/con")]
    #[case("a/COM1.json")]
    #[case("a/lpt9")]
    #[case("a/nome.")]
    #[case("a/nome ")]
    #[case("a/x?.json")]
    #[case("a/x:stream")]
    fn rejects_invalid_paths(#[case] rel: &str) {
        let err = validate_relative(rel).unwrap_err();
        assert_eq!(err.code(), "PATH_OUTSIDE_WORKSPACE", "{rel}");
    }

    #[test]
    fn reserved_names_ignore_case_and_extension() {
        assert!(is_reserved_name("CON"));
        assert!(is_reserved_name("nul.json"));
        assert!(is_reserved_name("com9.body.json"));
        assert!(!is_reserved_name("console"));
        assert!(!is_reserved_name("com10"));
    }

    #[test]
    fn length_limit() {
        assert!(check_length(&"a".repeat(MAX_RELATIVE_PATH_LEN)).is_ok());
        let err = check_length(&"a".repeat(MAX_RELATIVE_PATH_LEN + 1)).unwrap_err();
        assert_eq!(err.code(), "INVALID_NAME");
        assert!(err.to_string().contains("muito longo"));
    }

    #[test]
    fn relative_helpers() {
        assert_eq!(join_rel("", "a"), "a");
        assert_eq!(join_rel("a/b", "c.json"), "a/b/c.json");
        assert_eq!(parent_rel("a/b/c.json"), "a/b");
        assert_eq!(parent_rel("a"), "");
        assert_eq!(file_name_rel("a/b/c.json"), "c.json");
    }

    #[test]
    fn resolve_and_back_to_relative() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        let abs = resolve(&root, "a/b").unwrap();
        assert_eq!(to_relative(&root, &abs).unwrap(), "a/b");
        assert_eq!(resolve(&root, "").unwrap(), root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escaping_root() {
        let outside_dir = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        std::os::unix::fs::symlink(outside_dir.path(), root.join("fuga")).unwrap();
        assert_eq!(
            resolve(&root, "fuga").unwrap_err().code(),
            "PATH_OUTSIDE_WORKSPACE"
        );
    }

    #[test]
    fn is_inside_compares_by_component() {
        assert!(is_inside(Path::new("/w/root"), Path::new("/w/root/a")));
        assert!(!is_inside(Path::new("/w/root"), Path::new("/w/root2/a")));
        assert!(!is_inside(Path::new("/w/root"), Path::new("/w")));
    }

    #[cfg(windows)]
    #[test]
    fn is_inside_ignores_drive_letter_case_on_windows() {
        assert!(is_inside(
            Path::new(r"D:\workspace"),
            Path::new(r"d:\Workspace\x")
        ));
    }
}
