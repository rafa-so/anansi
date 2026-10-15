//! Utilitários compartilhados pelos testes de integração.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Copia uma fixture de `tests/fixtures/<nome>` para uma pasta temporária.
///
/// Arquivos e pastas que começam com `_` viram `.` na cópia (`_gitignore` →
/// `.gitignore`). Assim os pontos-arquivos das fixtures não afetam o git do
/// próprio `anansi`.
pub fn fixture(name: &str) -> (TempDir, PathBuf) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let tmp = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(tmp.path()).unwrap().join(name);
    copy_dir(&src, &root);
    (tmp, root)
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        let target_name = match name.strip_prefix('_') {
            Some(rest) => format!(".{rest}"),
            None => name,
        };
        let target = dst.join(target_name);
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Pasta temporária vazia (já canonicalizada).
pub fn empty_dir() -> (TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(tmp.path()).unwrap();
    (tmp, root)
}

pub fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap()
}

pub fn exists(root: &Path, rel: &str) -> bool {
    root.join(rel).exists()
}
