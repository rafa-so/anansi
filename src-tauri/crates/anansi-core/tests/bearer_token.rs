//! Bearer Token como segredo de ambiente (docs/bearer-token-local/SPEC.md).

mod common;

use anansi_core::env::{self, bearer::BEARER_REFERENCE, SecretAction};
use anansi_core::model::{Body, HttpMethod, KeyValue, RequestFile, Settings};
use anansi_core::Workspace;
use common::*;
use pretty_assertions::assert_eq;
use std::path::PathBuf;
use tempfile::TempDir;

const TOKEN: &str = "eyJhbGciOiJIUzI1NiJ9.e30.assinatura";

fn new_workspace() -> (TempDir, PathBuf, Workspace) {
    let (tmp, root) = empty_dir();
    let ws = Workspace::init(&root).unwrap();
    ws.collection_create("Store API", None).unwrap();
    env::create(&root, "hml").unwrap();
    (tmp, root, ws)
}

fn with_token(name: &str, value: &str) -> RequestFile {
    RequestFile {
        schema_version: 1,
        name: name.into(),
        method: HttpMethod::Get,
        url: "{{baseUrl}}/products".into(),
        query: vec![],
        headers: vec![KeyValue {
            key: "Authorization".into(),
            value: value.into(),
            enabled: true,
        }],
        body: Body::None,
        settings: Settings::default(),
    }
}

/// O arquivo versionado só tem a referência; o token fica no segredo local.
#[test]
fn saving_moves_the_token_to_the_secret() {
    let (_tmp, root, ws) = new_workspace();
    let saved = ws
        .request_create_in_env(
            "store-api",
            &with_token("Listar", &format!("Bearer {TOKEN}")),
            Some("hml"),
        )
        .unwrap();
    assert_eq!(saved.path, "store-api/listar.json");
    assert_eq!(saved.request.headers[0].value, BEARER_REFERENCE);
    let outcome = saved.bearer.unwrap();
    assert_eq!(
        (outcome.env.as_str(), outcome.action),
        ("hml", SecretAction::Created)
    );

    let on_disk = read(&root, "store-api/listar.json");
    assert!(on_disk.contains("Bearer {{bearer_token}}"));
    assert!(!on_disk.contains(TOKEN));
    assert_eq!(
        env::reveal_secret(&root, "hml", "bearer_token").unwrap(),
        TOKEN
    );
    assert_eq!(ws.request_get(&saved.path).unwrap(), saved.request);

    // Outro request com o mesmo token reaproveita o segredo.
    let other = ws
        .request_create_in_env(
            "store-api",
            &with_token("Criar", &format!("bearer {TOKEN}")),
            Some("hml"),
        )
        .unwrap();
    assert_eq!(other.bearer.unwrap().action, SecretAction::Unchanged);
}

/// Sem ambiente, nada é gravado: nem o request, nem o segredo.
#[test]
fn literal_token_without_environment_is_never_written() {
    let (_tmp, root, ws) = new_workspace();
    let err = ws
        .request_create("store-api", &with_token("Listar", "Bearer abc123"))
        .unwrap_err();
    assert_eq!(err.code(), "BEARER_NEEDS_ENVIRONMENT");
    assert!(!exists(&root, "store-api/listar.json"));

    let rel = ws
        .request_create("store-api", &with_token("Listar", BEARER_REFERENCE))
        .unwrap();
    let err = ws
        .request_save(&rel, &with_token("Listar renomeado", "Bearer abc123"))
        .unwrap_err();
    assert_eq!(err.code(), "BEARER_NEEDS_ENVIRONMENT");
    assert!(exists(&root, &rel), "o rename não aconteceu");
    assert!(!read(&root, &rel).contains("abc123"));
}

/// Requests antigos (ou editados fora do app) são convertidos pelo "Converter agora".
#[test]
fn convert_rewrites_legacy_files() {
    let (_tmp, root, ws) = new_workspace();
    for name in ["a", "b"] {
        std::fs::write(
            root.join(format!("store-api/{name}.json")),
            format!(
                r#"{{"schemaVersion":1,"name":"{name}","method":"GET","url":"https://x","headers":[{{"key":"Authorization","value":"Bearer {TOKEN}","enabled":true}}]}}"#
            ),
        )
        .unwrap();
    }
    assert!(env::bearer::json_has_literal(&read(
        &root,
        "store-api/a.json"
    )));
    let out = ws
        .convert_bearer(
            &["store-api/a.json".into(), "store-api/b.json".into()],
            Some("hml"),
        )
        .unwrap()
        .unwrap();
    assert_eq!(out.action, SecretAction::Created);
    for name in ["a", "b"] {
        let text = read(&root, &format!("store-api/{name}.json"));
        assert!(!text.contains(TOKEN), "{name}");
        assert!(!env::bearer::json_has_literal(&text));
    }
}

#[test]
fn convert_rejects_different_tokens() {
    let (_tmp, root, ws) = new_workspace();
    for (name, token) in [("a", "t1"), ("b", "t2")] {
        std::fs::write(
            root.join(format!("store-api/{name}.json")),
            format!(
                r#"{{"schemaVersion":1,"name":"{name}","method":"GET","url":"https://x","headers":[{{"key":"Authorization","value":"Bearer {token}"}}]}}"#
            ),
        )
        .unwrap();
    }
    let err = ws
        .convert_bearer(
            &["store-api/a.json".into(), "store-api/b.json".into()],
            Some("hml"),
        )
        .unwrap_err();
    assert_eq!(err.code(), "INVALID_REQUEST");
    assert!(
        read(&root, "store-api/a.json").contains("t1"),
        "nada foi alterado"
    );
}

/// Trocar de ambiente troca o token: cada ambiente tem o seu `bearer_token`.
#[test]
fn each_environment_resolves_its_own_token() {
    let (_tmp, root, ws) = new_workspace();
    env::create(&root, "prd").unwrap();
    let saved = ws
        .request_create_in_env(
            "store-api",
            &with_token("Listar", "Bearer token-hml"),
            Some("hml"),
        )
        .unwrap();
    ws.request_save_in_env(
        &saved.path,
        &with_token("Listar", "Bearer token-prd"),
        Some("prd"),
    )
    .unwrap();
    let request = ws.request_get(&saved.path).unwrap();
    for (name, expected) in [("hml", "Bearer token-hml"), ("prd", "Bearer token-prd")] {
        let data = env::load_environment(&root, name).unwrap();
        let (resolved, report) =
            env::resolver::Resolver::new(Some(&data)).resolve_request(&request);
        assert_eq!(resolved.headers[0].value, expected);
        assert!(report.used_secret && !report.unresolved.contains("bearer_token"));
    }
}
