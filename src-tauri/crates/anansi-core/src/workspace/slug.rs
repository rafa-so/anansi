//! Slugs e nomes de arquivo (RN-05, Seção 4.4 da spec).

use super::paths::is_reserved_name;
use std::collections::HashSet;
use std::path::Path;

/// Tamanho máximo do slug.
pub const MAX_SLUG_LEN: usize = 60;

/// Gera o nome em disco a partir do nome exibido.
pub fn slugify(name: &str) -> String {
    let ascii = deunicode::deunicode(name).to_lowercase();
    let mut slug = String::with_capacity(ascii.len());
    let mut last_dash = false;
    for c in ascii.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let mut slug: String = slug.trim_matches('-').chars().take(MAX_SLUG_LEN).collect();
    slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() || is_reserved_name(&slug) {
        slug.push_str(if slug.is_empty() { "item" } else { "-item" });
    }
    slug
}

/// Nomes existentes em um diretório, em minúsculas (o NTFS não diferencia maiúsculas).
pub fn existing_names(dir: &Path) -> HashSet<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

/// Escolhe um nome livre: `base + suffix`, depois `base-2 + suffix`, `base-3 + suffix`...
///
/// `taken` deve estar em minúsculas. `extra_suffixes` são outros nomes que o
/// item ocupa junto (ex.: o arquivo de body de um request).
pub fn unique_name(
    taken: &HashSet<String>,
    base: &str,
    suffix: &str,
    extra_suffixes: &[&str],
) -> String {
    let free = |stem: &str| {
        std::iter::once(suffix)
            .chain(extra_suffixes.iter().copied())
            .all(|s| !taken.contains(&format!("{stem}{s}").to_lowercase()))
    };
    if free(base) {
        return base.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if free(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("Listar Ações", "listar-acoes")]
    #[case("Listar produtos", "listar-produtos")]
    #[case("  GET /products?page=1  ", "get-products-page-1")]
    #[case("Store API", "store-api")]
    #[case("---", "item")]
    #[case("", "item")]
    #[case("💥", "boom")]
    #[case("CON", "con-item")]
    #[case("com1", "com1-item")]
    #[case("Lpt9", "lpt9-item")]
    #[case("Console", "console")]
    fn slugify_cases(#[case] name: &str, #[case] expected: &str) {
        assert_eq!(slugify(name), expected);
    }

    #[test]
    fn slugify_limits_length_without_trailing_dash() {
        let name = format!("{} fim", "a".repeat(59));
        let slug = slugify(&name);
        assert_eq!(slug.len(), 59, "o traço final é removido após o corte");
        assert!(!slug.ends_with('-'));
        assert!(slugify(&"b".repeat(100)).len() == MAX_SLUG_LEN);
    }

    #[test]
    fn unique_name_adds_numeric_suffix() {
        let taken: HashSet<String> = ["listar.json", "listar-2.json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(unique_name(&taken, "listar", ".json", &[]), "listar-3");
        assert_eq!(unique_name(&taken, "outro", ".json", &[]), "outro");
    }

    #[test]
    fn unique_name_ignores_case_like_ntfs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Listar-Produtos.json"), "{}").unwrap();
        let taken = existing_names(dir.path());
        assert_eq!(
            unique_name(&taken, "listar-produtos", ".json", &[]),
            "listar-produtos-2"
        );
    }

    #[test]
    fn unique_name_considers_body_files() {
        let taken: HashSet<String> = ["criar.body.json".to_string()].into_iter().collect();
        assert_eq!(
            unique_name(&taken, "criar", ".json", &[".body.json", ".body.txt"]),
            "criar-2"
        );
    }
}
