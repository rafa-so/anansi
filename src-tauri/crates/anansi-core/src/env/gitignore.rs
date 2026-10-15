//! Checagem e correção do `.gitignore` para os arquivos de segredos (RF-14, Seção 7.1).

use crate::error::AppResult;
use ignore::gitignore::GitignoreBuilder;
use std::path::Path;

pub const SECRETS_PATTERN: &str = "environments/*.secrets.json";
pub const TEMP_PATTERN: &str = ".*.tmp";

const FIX_BLOCK: &str = "# anansi: segredos locais por ambiente\nenvironments/*.secrets.json\n";

/// `true` quando o `.gitignore` da raiz ignora os arquivos de segredos.
pub fn status(root: &Path) -> bool {
    let file = root.join(".gitignore");
    if !file.exists() {
        return false;
    }
    let mut builder = GitignoreBuilder::new(root);
    if builder.add(&file).is_some() {
        return false;
    }
    let Ok(matcher) = builder.build() else {
        return false;
    };
    [
        "environments/local.secrets.json",
        "environments/x.secrets.json",
    ]
    .iter()
    .all(|p| {
        matcher
            .matched_path_or_any_parents(root.join(p), false)
            .is_ignore()
    })
}

/// Acrescenta o padrão de segredos ao final do `.gitignore` (criando o arquivo se preciso).
/// Não faz nada se o padrão já funciona.
pub fn fix(root: &Path) -> AppResult<()> {
    if status(root) {
        return Ok(());
    }
    let file = root.join(".gitignore");
    let mut text = std::fs::read_to_string(&file).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(FIX_BLOCK);
    crate::workspace::fs_ops::write_atomic(&file, text.as_bytes()).map(|_| ())
}

/// Conteúdo do `.gitignore` criado ao inicializar um workspace.
pub fn initial_content() -> String {
    format!("{FIX_BLOCK}\n# anansi: temporários da escrita atômica\n{TEMP_PATTERN}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn with_gitignore(content: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        if let Some(c) = content {
            std::fs::write(dir.path().join(".gitignore"), c).unwrap();
        }
        dir
    }

    #[rstest]
    #[case("environments/*.secrets.json\n", true)]
    #[case("*.secrets.json\n", true)]
    #[case("environments/\n", true)]
    #[case("node_modules\n", false)]
    #[case("environments/local.secrets.json\n", false)]
    #[case("", false)]
    fn status_accepts_equivalent_patterns(#[case] content: &str, #[case] expected: bool) {
        let dir = with_gitignore(Some(content));
        assert_eq!(status(dir.path()), expected, "{content:?}");
    }

    #[test]
    fn status_is_false_without_file() {
        let dir = with_gitignore(None);
        assert!(!status(dir.path()));
    }

    #[test]
    fn fix_creates_or_appends_and_is_idempotent() {
        let dir = with_gitignore(Some("node_modules"));
        fix(dir.path()).unwrap();
        assert!(status(dir.path()));
        let first = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(first.starts_with("node_modules\n# anansi"));
        fix(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
            first
        );

        let empty = with_gitignore(None);
        fix(empty.path()).unwrap();
        assert!(status(empty.path()));
    }

    #[test]
    fn initial_content_ignores_secrets_and_temp_files() {
        let dir = with_gitignore(Some(&initial_content()));
        assert!(status(dir.path()));
        let mut builder = GitignoreBuilder::new(dir.path());
        builder.add(dir.path().join(".gitignore"));
        let matcher = builder.build().unwrap();
        assert!(matcher
            .matched(dir.path().join("c/.x.json.tmp"), false)
            .is_ignore());
    }
}
