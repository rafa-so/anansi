//! Monta a árvore do workspace a partir do disco (RF-02, RN-10, RN-11, Seção 4.2).

use super::paths::join_rel;
use crate::env::ENVIRONMENTS_DIR;
use crate::error::AppError;
use crate::model::schema::load;
use crate::model::{
    is_body_file_name, CollectionFile, DiskRequest, TreeNode, COLLECTION_SCHEMA_VERSION,
    REQUEST_SCHEMA_VERSION,
};
use std::path::Path;

pub const COLLECTION_FILE: &str = "collection.json";

/// Árvore completa, já ordenada.
pub fn scan(root: &Path) -> Vec<TreeNode> {
    let mut collections = Vec::new();
    for entry in sorted_entries(root) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.eq_ignore_ascii_case(ENVIRONMENTS_DIR) || !is_dir(&entry) {
            continue;
        }
        let dir = entry.path();
        if !dir.join(COLLECTION_FILE).is_file() {
            continue;
        }
        collections.push(scan_collection(&dir, &name));
    }
    sort_nodes(&mut collections);
    collections
}

/// Lê uma collection (diretório de primeiro nível com `collection.json`).
pub fn scan_collection(dir: &Path, rel: &str) -> TreeNode {
    let (name, description, error) = match std::fs::read_to_string(dir.join(COLLECTION_FILE)) {
        Ok(text) => match load::<CollectionFile>(&text, COLLECTION_SCHEMA_VERSION, &[]) {
            Ok(loaded) => (loaded.value.name, loaded.value.description, None),
            Err(e) => (rel.to_string(), None, Some(e.to_string())),
        },
        Err(e) => (
            rel.to_string(),
            None,
            Some(AppError::io(&e, dir).to_string()),
        ),
    };
    TreeNode::Collection {
        path: rel.to_string(),
        name,
        description,
        children: scan_children(dir, rel),
        error,
    }
}

/// Filhos (pastas e requests) de um diretório dentro de uma collection.
pub fn scan_children(dir: &Path, rel: &str) -> Vec<TreeNode> {
    let mut nodes = Vec::new();
    for entry in sorted_entries(dir) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let child_rel = join_rel(rel, &name);
        if is_dir(&entry) {
            nodes.push(TreeNode::Folder {
                path: child_rel.clone(),
                name: name.clone(),
                children: scan_children(&entry.path(), &child_rel),
            });
        } else if is_request_file_name(&name, rel) {
            nodes.push(read_request_node(&entry.path(), child_rel));
        }
    }
    sort_nodes(&mut nodes);
    nodes
}

/// Um arquivo é request quando termina em `.json`, não é de body e não é o
/// `collection.json` da raiz da collection.
pub fn is_request_file_name(name: &str, parent_rel: &str) -> bool {
    let lower = name.to_lowercase();
    if !lower.ends_with(".json") || name.starts_with('.') || is_body_file_name(name) {
        return false;
    }
    !(lower == COLLECTION_FILE && !parent_rel.contains('/'))
}

/// Lê só o necessário para o nó da árvore. Erros viram nó com `error`.
pub fn read_request_node(path: &Path, rel: String) -> TreeNode {
    let file_name = rel.rsplit('/').next().unwrap_or(&rel).to_string();
    let error_node = |error: AppError| TreeNode::Request {
        read_only: error.code() == "UNSUPPORTED_SCHEMA_VERSION",
        path: rel.clone(),
        name: file_name.clone(),
        method: String::new(),
        url: String::new(),
        error: Some(error.to_string()),
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return error_node(AppError::io(&e, path)),
    };
    match load::<DiskRequest>(&text, REQUEST_SCHEMA_VERSION, &[]) {
        Ok(loaded) => {
            let r = loaded.value;
            TreeNode::Request {
                path: rel,
                name: r.name,
                method: r.method.as_str().into(),
                url: r.url,
                error: None,
                read_only: false,
            }
        }
        Err(e) => error_node(e),
    }
}

fn sorted_entries(dir: &Path) -> Vec<std::fs::DirEntry> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map(|it| it.filter_map(|e| e.ok()).collect())
        .unwrap_or_default();
    entries.sort_by_key(|e| e.file_name());
    entries
}

fn is_dir(entry: &std::fs::DirEntry) -> bool {
    entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
}

/// Chave de ordenação: sem acentos e sem diferenciar maiúsculas.
pub fn sort_key(name: &str) -> String {
    deunicode::deunicode(name).to_lowercase()
}

/// RN-11: pastas antes dos requests; dentro de cada grupo, ordem alfabética do nome exibido.
pub fn sort_nodes(nodes: &mut [TreeNode]) {
    nodes.sort_by(|a, b| {
        let group = |n: &TreeNode| matches!(n, TreeNode::Request { .. }) as u8;
        group(a)
            .cmp(&group(b))
            .then_with(|| sort_key(a.name()).cmp(&sort_key(b.name())))
            .then_with(|| a.path().cmp(b.path()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_file_names() {
        assert!(is_request_file_name("listar.json", "store-api"));
        assert!(!is_request_file_name("collection.json", "store-api"));
        assert!(
            is_request_file_name("collection.json", "store-api/products"),
            "só o da raiz da collection é especial"
        );
        assert!(!is_request_file_name("listar.body.json", "store-api"));
        assert!(!is_request_file_name(".listar.json.tmp", "store-api"));
        assert!(!is_request_file_name("notas.txt", "store-api"));
    }

    #[test]
    fn sort_key_ignores_accents_and_case() {
        assert_eq!(sort_key("Ação"), "acao");
        let mut v = vec!["b", "Á", "a", "C"];
        v.sort_by_key(|s| sort_key(s));
        assert_eq!(v, vec!["Á", "a", "b", "C"]);
    }
}
