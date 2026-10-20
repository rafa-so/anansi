//! CRUD de collections, pastas e requests (E3: RF-03 a RF-07, RF-18, RN-05, RNF-04).

mod common;

use anansi_core::model::{Body, FormField, HttpMethod, KeyValue, RequestFile, Settings, TreeNode};
use anansi_core::Workspace;
use common::*;
use pretty_assertions::assert_eq;
use std::path::PathBuf;
use tempfile::TempDir;

fn new_workspace() -> (TempDir, PathBuf, Workspace) {
    let (tmp, root) = empty_dir();
    let ws = Workspace::init(&root).unwrap();
    (tmp, root, ws)
}

fn request(name: &str) -> RequestFile {
    RequestFile {
        schema_version: 1,
        name: name.into(),
        method: HttpMethod::Get,
        url: "{{baseUrlStore}}/products".into(),
        query: vec![],
        headers: vec![],
        body: Body::None,
        settings: Settings::default(),
    }
}

fn kv(key: &str, value: &str) -> KeyValue {
    KeyValue {
        key: key.into(),
        value: value.into(),
        enabled: true,
    }
}

/// V-03: collection, pasta e pasta aninhada.
#[test]
fn creates_collection_and_nested_folders() {
    let (_tmp, root, ws) = new_workspace();
    let collection = ws
        .collection_create("Store API", Some("Endpoints do store-api"))
        .unwrap();
    assert_eq!(collection.path(), "store-api");
    assert_eq!(
        read(&root, "store-api/collection.json"),
        "{\n  \"schemaVersion\": 1,\n  \"name\": \"Store API\",\n  \"description\": \"Endpoints do store-api\"\n}\n"
    );
    let products = ws.folder_create("store-api", "products").unwrap();
    let reviews = ws.folder_create(products.path(), "reviews").unwrap();
    assert_eq!(reviews.path(), "store-api/products/reviews");
    assert!(exists(&root, "store-api/products/.gitkeep"));
    assert!(exists(&root, "store-api/products/reviews/.gitkeep"));
    let tree = ws.tree();
    assert_eq!(
        tree[0].children()[0].children()[0].path(),
        "store-api/products/reviews"
    );
}

#[test]
fn collection_without_description_omits_field_and_empty_name_is_rejected() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("Billing API", Some("   ")).unwrap();
    assert_eq!(
        read(&root, "billing-api/collection.json"),
        "{\n  \"schemaVersion\": 1,\n  \"name\": \"Billing API\"\n}\n"
    );
    assert_eq!(
        ws.collection_create("  ", None).unwrap_err().code(),
        "INVALID_NAME"
    );
}

#[test]
fn collection_named_environments_does_not_clash_with_environments_folder() {
    let (_tmp, _root, ws) = new_workspace();
    assert_eq!(
        ws.collection_create("Environments", None).unwrap().path(),
        "environments-2"
    );
}

#[test]
fn renames_collection_and_folder() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("Store API", None).unwrap();
    ws.folder_create("store-api", "products").unwrap();
    let renamed = ws
        .collection_update("store-api", "Cliente API", Some("nova"))
        .unwrap();
    assert_eq!(renamed.path(), "cliente-api");
    assert!(exists(&root, "cliente-api/products/.gitkeep"));
    assert!(!exists(&root, "store-api"));
    // Mudar só maiúsculas não renomeia o diretório.
    assert_eq!(
        ws.collection_update("cliente-api", "CLIENTE API", None)
            .unwrap()
            .path(),
        "cliente-api"
    );
    let folder = ws.folder_rename("cliente-api/products", "Produtos").unwrap();
    assert_eq!(folder.path(), "cliente-api/produtos");
    assert_eq!(
        ws.folder_rename("cliente-api/produtos", "PRODUTOS")
            .unwrap()
            .path(),
        "cliente-api/produtos"
    );
}

#[test]
fn folder_name_collision_gets_suffix() {
    let (_tmp, _root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    assert_eq!(ws.folder_create("c", "Pasta").unwrap().path(), "c/pasta");
    assert_eq!(ws.folder_create("c", "pasta").unwrap().path(), "c/pasta-2");
}

/// V-04: request "Listar Ações" vira `listar-acoes.json`, com LF e quebra final.
#[test]
fn creates_request_with_stable_format() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("Store API", None).unwrap();
    ws.folder_create("store-api", "products").unwrap();
    let mut req = request("Listar Ações");
    req.query = vec![kv("page", "1")];
    req.headers = vec![kv("Authorization", "Bearer {{token}}")];
    req.settings.verify_tls = Some(false);
    let rel = ws.request_create("store-api/products", &req).unwrap();
    assert_eq!(rel, "store-api/products/listar-acoes.json");
    let text = read(&root, &rel);
    assert!(!text.contains('\r'));
    assert!(text.ends_with("}\n"));
    insta::assert_snapshot!("request_listar_acoes", text);
    assert_eq!(ws.request_get(&rel).unwrap(), req);
}

/// V-05: salvar sem mudanças não regrava o arquivo.
#[test]
fn saving_unchanged_request_does_not_touch_file() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    let rel = ws.request_create("c", &request("Listar")).unwrap();
    let before = std::fs::metadata(root.join(&rel))
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    let loaded = ws.request_get(&rel).unwrap();
    assert_eq!(ws.request_save(&rel, &loaded).unwrap(), rel);
    assert_eq!(
        std::fs::metadata(root.join(&rel))
            .unwrap()
            .modified()
            .unwrap(),
        before
    );
}

/// V-07: renomear o request renomeia o arquivo (e o body junto).
#[test]
fn renaming_request_renames_files() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    let mut req = request("Listar Ações");
    req.body = Body::Json {
        content: "{}".into(),
    };
    let rel = ws.request_create("c", &req).unwrap();
    req.name = "Listar produtos".into();
    let new_rel = ws.request_save(&rel, &req).unwrap();
    assert_eq!(new_rel, "c/listar-produtos.json");
    assert!(!exists(&root, "c/listar-acoes.json"));
    assert!(!exists(&root, "c/listar-acoes.body.json"));
    assert_eq!(read(&root, "c/listar-produtos.body.json"), "{}");
    // Só maiúsculas: mesmo arquivo.
    req.name = "LISTAR PRODUTOS".into();
    assert_eq!(ws.request_save(&new_rel, &req).unwrap(), new_rel);
}

/// V-08 e V-27: colisões, inclusive com arquivo criado à mão com maiúsculas.
#[test]
fn request_name_collisions() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    assert_eq!(
        ws.request_create("c", &request("Listar")).unwrap(),
        "c/listar.json"
    );
    assert_eq!(
        ws.request_create("c", &request("Listar")).unwrap(),
        "c/listar-2.json"
    );
    std::fs::write(root.join("c/Listar-Produtos.json"), "{}").unwrap();
    assert_eq!(
        ws.request_create("c", &request("Listar produtos")).unwrap(),
        "c/listar-produtos-2.json"
    );
}

#[test]
fn renaming_onto_existing_name_gets_suffix() {
    let (_tmp, _root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    ws.request_create("c", &request("A")).unwrap();
    let b = ws.request_create("c", &request("B")).unwrap();
    assert_eq!(ws.request_save(&b, &request("A")).unwrap(), "c/a-2.json");
}

/// P-01: body json/text em arquivo separado, form-urlencoded inline.
#[test]
fn body_files_follow_body_type() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    let mut req = request("Criar produto");
    req.method = HttpMethod::Post;
    req.body = Body::Json {
        content: "{\r\n  \"id\": {{produtoId}}\r\n}".into(),
    };
    let rel = ws.request_create("c", &req).unwrap();
    insta::assert_snapshot!("request_criar_produto", read(&root, &rel));
    assert_eq!(
        read(&root, "c/criar-produto.body.json"),
        "{\n  \"id\": {{produtoId}}\n}"
    );

    req.body = Body::Text {
        content: "texto".into(),
    };
    ws.request_save(&rel, &req).unwrap();
    assert!(
        !exists(&root, "c/criar-produto.body.json"),
        "trocar o tipo remove o body antigo"
    );
    assert_eq!(read(&root, "c/criar-produto.body.txt"), "texto");

    req.body = Body::FormUrlencoded {
        fields: vec![
            FormField::text("a", "1"),
            FormField::file("anexo", "C:/docs/a.pdf"),
        ],
    };
    ws.request_save(&rel, &req).unwrap();
    assert!(!exists(&root, "c/criar-produto.body.txt"));
    assert!(read(&root, &rel).contains("\"form-urlencoded\""));
    assert!(
        read(&root, &rel).contains("\"type\": \"file\""),
        "campo de arquivo grava o tipo"
    );
    assert_eq!(ws.request_get(&rel).unwrap().body, req.body);

    req.body = Body::None;
    ws.request_save(&rel, &req).unwrap();
    assert!(!read(&root, &rel).contains("body"));
}

#[test]
fn body_file_missing_reads_as_empty_and_body_files_are_hidden() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    let mut req = request("X");
    req.body = Body::Json {
        content: "{}".into(),
    };
    let rel = ws.request_create("c", &req).unwrap();
    std::fs::remove_file(root.join("c/x.body.json")).unwrap();
    assert_eq!(
        ws.request_get(&rel).unwrap().body,
        Body::Json {
            content: String::new()
        }
    );
    std::fs::write(root.join("c/orfao.body.txt"), "x").unwrap();
    let names: Vec<_> = ws.tree()[0]
        .children()
        .iter()
        .map(|n| n.path().to_string())
        .collect();
    assert_eq!(names, vec!["c/x.json"]);
}

#[test]
fn body_file_name_is_reserved_for_collision() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    std::fs::write(root.join("c/criar.body.json"), "{}").unwrap();
    assert_eq!(
        ws.request_create("c", &request("Criar")).unwrap(),
        "c/criar-2.json"
    );
}

/// V-20: duplicar.
#[test]
fn duplicates_request_with_body() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    let mut req = request("Criar produto");
    req.body = Body::Json {
        content: "{\"a\":1}".into(),
    };
    let rel = ws.request_create("c", &req).unwrap();
    let copy = ws.request_duplicate(&rel).unwrap();
    assert_eq!(copy, "c/criar-produto-copia.json");
    let loaded = ws.request_get(&copy).unwrap();
    assert_eq!(loaded.name, "Criar produto (cópia)");
    assert_eq!(read(&root, "c/criar-produto-copia.body.json"), "{\"a\":1}");
}

#[test]
fn deletes_request_folder_and_collection() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    ws.folder_create("c", "p").unwrap();
    let mut req = request("X");
    req.body = Body::Text {
        content: "t".into(),
    };
    let rel = ws.request_create("c/p", &req).unwrap();
    ws.request_create("c/p", &request("Y")).unwrap();
    ws.request_create("c", &request("Z")).unwrap();
    assert_eq!(ws.request_count("c").unwrap(), 3);
    assert_eq!(ws.request_count("c/p").unwrap(), 2);
    assert_eq!(ws.request_count(&rel).unwrap(), 1);

    ws.request_delete(&rel).unwrap();
    assert!(!exists(&root, "c/p/x.json"));
    assert!(!exists(&root, "c/p/x.body.txt"));
    assert_eq!(ws.request_delete(&rel).unwrap_err().code(), "NOT_FOUND");
    ws.folder_delete("c/p").unwrap();
    assert!(!exists(&root, "c/p"));
    ws.collection_delete("c").unwrap();
    assert!(ws.tree().is_empty());
}

/// Aba "removido do disco": salvar recria o arquivo.
#[test]
fn saving_removed_request_recreates_it() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    let rel = ws.request_create("c", &request("X")).unwrap();
    std::fs::remove_file(root.join(&rel)).unwrap();
    assert_eq!(ws.request_save(&rel, &request("X")).unwrap(), rel);
    assert!(exists(&root, &rel));
}

#[test]
fn newer_schema_is_read_only() {
    let (_tmp, root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    std::fs::write(
        root.join("c/futuro.json"),
        r#"{"schemaVersion":9,"name":"F","method":"GET","url":"x"}"#,
    )
    .unwrap();
    assert_eq!(
        ws.request_get("c/futuro.json").unwrap_err().code(),
        "UNSUPPORTED_SCHEMA_VERSION"
    );
    assert_eq!(
        ws.request_save("c/futuro.json", &request("F"))
            .unwrap_err()
            .code(),
        "UNSUPPORTED_SCHEMA_VERSION"
    );
    assert!(matches!(
        &ws.tree()[0].children()[0],
        TreeNode::Request {
            read_only: true,
            ..
        }
    ));
}

#[test]
fn operations_check_item_kind_and_path() {
    let (_tmp, _root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    ws.folder_create("c", "p").unwrap();
    let rel = ws.request_create("c", &request("X")).unwrap();
    assert_eq!(ws.collection_delete("c/p").unwrap_err().code(), "NOT_FOUND");
    assert_eq!(
        ws.folder_delete("c").unwrap_err().code(),
        "NOT_FOUND",
        "collection não é pasta"
    );
    assert_eq!(
        ws.folder_create(&rel, "x").unwrap_err().code(),
        "NOT_FOUND",
        "request não recebe filhos"
    );
    assert_eq!(
        ws.folder_create("environments", "x").unwrap_err().code(),
        "NOT_FOUND"
    );
    assert_eq!(
        ws.request_create("", &request("X")).unwrap_err().code(),
        "NOT_FOUND",
        "request fora de collection"
    );
    assert_eq!(
        ws.request_get("c/collection.json").unwrap_err().code(),
        "NOT_FOUND"
    );
    assert_eq!(
        ws.request_get("../fora.json").unwrap_err().code(),
        "PATH_OUTSIDE_WORKSPACE"
    );
    assert_eq!(
        ws.folder_create("c/../..", "x").unwrap_err().code(),
        "PATH_OUTSIDE_WORKSPACE"
    );
}

/// V-29: caminho relativo acima de 180 caracteres é bloqueado.
#[test]
fn rejects_too_long_paths() {
    let (_tmp, _root, ws) = new_workspace();
    ws.collection_create("C", None).unwrap();
    let mut parent = "c".to_string();
    let segment = "a".repeat(55);
    for i in 0..3 {
        parent = ws
            .folder_create(&parent, &format!("{segment}{i}"))
            .unwrap()
            .path()
            .to_string();
    }
    let err = ws
        .folder_create(&parent, &format!("{segment}x"))
        .unwrap_err();
    assert_eq!(err.code(), "INVALID_NAME");
    assert!(err.to_string().contains("muito longo"));
}
