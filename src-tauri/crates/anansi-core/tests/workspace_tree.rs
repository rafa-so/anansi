//! Abertura, inicialização e árvore do workspace (E2: RF-01, RF-02, RN-10, RN-11).

mod common;

use common::*;
use anansi_core::model::TreeNode;
use anansi_core::Workspace;
use pretty_assertions::assert_eq;

/// Resumo legível da árvore: uma linha por nó, com indentação.
fn outline(nodes: &[TreeNode], depth: usize, out: &mut Vec<String>) {
    for node in nodes {
        let pad = "  ".repeat(depth);
        let line = match node {
            TreeNode::Collection {
                name, path, error, ..
            } => format!("{pad}[C] {name} ({path}){}", err(error)),
            TreeNode::Folder { name, path, .. } => format!("{pad}[F] {name} ({path})"),
            TreeNode::Request {
                name,
                path,
                method,
                error,
                read_only,
                ..
            } => {
                format!(
                    "{pad}[R] {method} {name} ({path}){}{}",
                    err(error),
                    if *read_only { " ro" } else { "" }
                )
            }
        };
        out.push(line);
        outline(node.children(), depth + 1, out);
    }
}

fn err(error: &Option<String>) -> &'static str {
    if error.is_some() {
        " !erro"
    } else {
        ""
    }
}

fn tree_outline(ws: &Workspace) -> Vec<String> {
    let mut out = vec![];
    outline(&ws.tree(), 0, &mut out);
    out
}

#[test]
fn scans_fixture_with_order_ignores_and_errors() {
    let (_tmp, root) = fixture("workspace-basic");
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(root.join(".git/HEAD"), "x").unwrap();
    let ws = Workspace::open(&root).unwrap();
    assert_eq!(
        tree_outline(&ws),
        vec![
            "[C] Billing API (billing-api)",
            "[C] Store API (store-api)",
            "  [F] products (store-api/products)",
            "    [F] reviews (store-api/products/reviews)",
            "      [R] DELETE Ações do cliente (store-api/products/reviews/acoes.json)",
            "    [R] POST Criar produto (store-api/products/criar-produto.json)",
            "    [R]  futuro.json (store-api/products/futuro.json) !erro ro",
            "    [R] GET Listar produtos (store-api/products/listar-produtos.json)",
            "    [R]  quebrado.json (store-api/products/quebrado.json) !erro",
            "  [R] GET Health check (store-api/health-check.json)",
        ]
    );
}

#[test]
fn error_nodes_explain_the_problem() {
    let (_tmp, root) = fixture("workspace-basic");
    let ws = Workspace::open(&root).unwrap();
    let store = ws
        .tree()
        .into_iter()
        .find(|n| n.path() == "store-api")
        .unwrap();
    let products = &store.children()[0];
    let messages: Vec<String> = products
        .children()
        .iter()
        .filter_map(|n| match n {
            TreeNode::Request { error: Some(e), .. } => Some(e.clone()),
            _ => None,
        })
        .collect();
    assert!(messages[0].contains("versão mais nova"), "{messages:?}");
    assert!(messages[1].contains("JSON inválido"), "{messages:?}");
}

#[test]
fn info_lists_environments_and_gitignore_status() {
    let (_tmp, root) = fixture("workspace-basic");
    let info = Workspace::open(&root).unwrap().info().unwrap();
    assert_eq!(info.name, "Demo");
    assert_eq!(info.environments, vec!["hml", "local"]);
    assert!(info.gitignore_ok);
}

#[test]
fn open_rejects_folder_without_workspace_file() {
    let (_tmp, root) = empty_dir();
    assert_eq!(
        Workspace::open(&root).unwrap_err().code(),
        "NOT_A_WORKSPACE"
    );
    assert_eq!(
        Workspace::open(&root.join("nao-existe"))
            .unwrap_err()
            .code(),
        "NOT_FOUND"
    );
}

#[test]
fn open_rejects_invalid_workspace_file() {
    let (_tmp, root) = empty_dir();
    std::fs::write(root.join("workspace.json"), "{").unwrap();
    assert_eq!(Workspace::open(&root).unwrap_err().code(), "INVALID_FILE");
}

/// V-01: abrir uma pasta vazia e inicializar.
#[test]
fn init_creates_minimal_structure() {
    let (_tmp, parent) = empty_dir();
    let root = parent.join("minhas-collections");
    std::fs::create_dir(&root).unwrap();
    let ws = Workspace::init(&root).unwrap();
    assert_eq!(
        read(&root, "workspace.json"),
        "{\n  \"schemaVersion\": 1,\n  \"name\": \"minhas-collections\"\n}\n"
    );
    assert!(exists(&root, "environments/.gitkeep"));
    assert_eq!(read(&root, ".gitattributes"), "* text=auto eol=lf\n");
    assert!(read(&root, ".gitignore").contains("environments/*.secrets.json"));
    assert!(read(&root, ".gitignore").contains(".*.tmp"));
    let info = ws.info().unwrap();
    assert!(info.gitignore_ok);
    assert!(info.environments.is_empty());
    assert!(ws.tree().is_empty());
}

#[test]
fn init_preserves_existing_files() {
    let (_tmp, root) = empty_dir();
    std::fs::write(root.join(".gitignore"), "node_modules\n").unwrap();
    std::fs::write(root.join(".gitattributes"), "*.png binary\n").unwrap();
    Workspace::init(&root).unwrap();
    let gitignore = read(&root, ".gitignore");
    assert!(gitignore.starts_with("node_modules\n"));
    assert!(gitignore.contains("environments/*.secrets.json"));
    assert_eq!(read(&root, ".gitattributes"), "*.png binary\n");
    // Inicializar de novo não muda nada.
    Workspace::init(&root).unwrap();
    assert_eq!(read(&root, ".gitignore"), gitignore);
}

#[test]
fn tree_paths_use_forward_slashes() {
    let (_tmp, root) = fixture("workspace-basic");
    let ws = Workspace::open(&root).unwrap();
    let mut out = vec![];
    fn collect(nodes: &[TreeNode], out: &mut Vec<String>) {
        for n in nodes {
            out.push(n.path().to_string());
            collect(n.children(), out);
        }
    }
    collect(&ws.tree(), &mut out);
    assert!(out.iter().all(|p| !p.contains('\\')));
}

#[test]
fn tree_serializes_like_the_spec() {
    let (_tmp, root) = fixture("workspace-basic");
    let ws = Workspace::open(&root).unwrap();
    let admin = &ws.tree()[0];
    assert_eq!(
        serde_json::to_value(admin).unwrap(),
        serde_json::json!({ "kind": "collection", "path": "billing-api", "name": "Billing API", "children": [] })
    );
}
