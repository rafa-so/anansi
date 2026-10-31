//! Bearer Token como segredo de ambiente (docs/bearer-token-local/SPEC.md).
//!
//! Um header `Authorization: Bearer <token literal>` nunca vai para o arquivo
//! do request: o token vira o segredo `bearer_token` do ambiente
//! (`environments/<nome>.secrets.json`, fora do git) e o header passa a ser
//! `Bearer {{bearer_token}}`. Valores que já têm `{{...}}` não são tocados.

use super::{store_secret, SecretAction};
use crate::error::{AppError, AppResult};
use crate::model::RequestFile;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

/// Nome do segredo que guarda o token.
pub const BEARER_SECRET_NAME: &str = "bearer_token";

/// Valor gravado no header no lugar do token.
pub const BEARER_REFERENCE: &str = "Bearer {{bearer_token}}";

/// O que aconteceu com o segredo ao guardar o token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BearerOutcome {
    /// Ambiente onde o segredo foi guardado.
    pub env: String,
    pub action: SecretAction,
    /// `true` quando o `.gitignore` foi corrigido para proteger os segredos.
    pub gitignore_fixed: bool,
}

fn is_authorization(key: &str) -> bool {
    key.trim().eq_ignore_ascii_case("authorization")
}

/// Token literal de um valor `Bearer <token>`. `None` se não for Bearer, se o
/// token estiver vazio ou se já tiver `{{` (referência a variável).
pub fn literal_token(value: &str) -> Option<&str> {
    let value = value.trim();
    let scheme = value.get(..6)?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let rest = &value[6..];
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let token = rest.trim();
    if token.is_empty() || token.contains("{{") {
        return None;
    }
    Some(token)
}

/// Tokens literais distintos nos headers `Authorization` (habilitados ou não), na ordem.
pub fn literal_tokens(request: &RequestFile) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    for h in request.headers.iter().filter(|h| is_authorization(&h.key)) {
        if let Some(t) = literal_token(&h.value) {
            if !tokens.iter().any(|x| x == t) {
                tokens.push(t.to_string());
            }
        }
    }
    tokens
}

pub fn has_literal(request: &RequestFile) -> bool {
    !literal_tokens(request).is_empty()
}

/// Troca os tokens literais por `Bearer {{bearer_token}}`. Também usado no
/// "Copiar como cURL" sem valores, para um rascunho ainda não salvo.
pub fn replace_literals(request: &mut RequestFile) {
    for h in request
        .headers
        .iter_mut()
        .filter(|h| is_authorization(&h.key))
    {
        if literal_token(&h.value).is_some() {
            h.value = BEARER_REFERENCE.to_string();
        }
    }
}

/// O único token literal do request; erro se houver mais de um diferente.
pub fn single_token(request: &RequestFile) -> AppResult<Option<String>> {
    let mut tokens = literal_tokens(request);
    match tokens.len() {
        0 => Ok(None),
        1 => Ok(tokens.pop()),
        _ => Err(AppError::InvalidRequest(format!(
            "Este request tem tokens Bearer diferentes; o segredo {BEARER_SECRET_NAME} guarda só um. Deixe um só header Authorization com token."
        ))),
    }
}

fn needs_environment() -> AppError {
    AppError::BearerNeedsEnvironment(format!(
        "Escolha um ambiente para guardar o token Bearer como o segredo {BEARER_SECRET_NAME}."
    ))
}

/// Guarda o token literal (se houver) no segredo `bearer_token` do ambiente e
/// troca o header pela referência. Sem token literal, não faz nada.
///
/// Ordem: `.gitignore` (se preciso) → segredo → (quem chamou grava o request).
pub fn extract(
    root: &Path,
    request: &mut RequestFile,
    env: Option<&str>,
) -> AppResult<Option<BearerOutcome>> {
    let Some(token) = single_token(request)? else {
        return Ok(None);
    };
    let env = env.ok_or_else(needs_environment)?;
    let (action, gitignore_fixed) = store_secret(root, env, BEARER_SECRET_NAME, &token)?;
    replace_literals(request);
    Ok(Some(BearerOutcome {
        env: env.to_string(),
        action,
        gitignore_fixed,
    }))
}

/// O que `extract` faria, sem gravar nada (prévia do "Importar cURL").
/// Ambiente que ainda não existe conta como `created`.
pub fn preview(
    root: &Path,
    request: &RequestFile,
    env: Option<&str>,
) -> AppResult<Option<SecretAction>> {
    let Some(token) = single_token(request)? else {
        return Ok(None);
    };
    let env = env.ok_or_else(needs_environment)?;
    super::secret_action(root, env, BEARER_SECRET_NAME, &token).map(Some)
}

/// `true` se o JSON tem algum objeto `{ "key": "Authorization", "value": "Bearer <literal>" }`.
/// Usado na varredura do push, que lê arquivos do disco (inclusive editados fora do app).
pub fn json_has_literal(text: &str) -> bool {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    fn walk(v: &Value) -> bool {
        match v {
            Value::Object(map) => {
                let key = map.get("key").and_then(Value::as_str);
                let val = map.get("value").and_then(Value::as_str);
                if let (Some(k), Some(v)) = (key, val) {
                    if is_authorization(k) && literal_token(v).is_some() {
                        return true;
                    }
                }
                map.values().any(walk)
            }
            Value::Array(items) => items.iter().any(walk),
            _ => false,
        }
    }
    walk(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{self, gitignore};
    use crate::model::{HttpMethod, KeyValue, Settings};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    fn kv(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }

    fn request(headers: Vec<KeyValue>) -> RequestFile {
        RequestFile {
            schema_version: 1,
            name: "X".into(),
            method: HttpMethod::Get,
            url: "https://x".into(),
            query: vec![],
            headers,
            body: Default::default(),
            settings: Settings::default(),
        }
    }

    fn workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".gitignore"),
            "environments/*.secrets.json\n",
        )
        .unwrap();
        dir
    }

    #[rstest]
    #[case("Bearer abc", Some("abc"))]
    #[case("  bearer   eyJ.x-y_z  ", Some("eyJ.x-y_z"))]
    #[case("BEARER abc", Some("abc"))]
    #[case("Bearer {{bearer_token}}", None)]
    #[case("Bearer abc{{x}}", None)]
    #[case("Bearer ", None)]
    #[case("Bearerabc", None)]
    #[case("Basic YWRtaW4=", None)]
    #[case("", None)]
    fn literal_token_cases(#[case] value: &str, #[case] expected: Option<&str>) {
        assert_eq!(literal_token(value), expected, "{value:?}");
    }

    #[test]
    fn only_authorization_headers_count_even_disabled() {
        let mut off = kv("authorization", "Bearer b");
        off.enabled = false;
        let req = request(vec![
            kv("X-Token", "Bearer a"),
            off,
            kv("Authorization", "Bearer b"),
        ]);
        assert_eq!(literal_tokens(&req), vec!["b"]);
        let mut converted = req.clone();
        replace_literals(&mut converted);
        assert_eq!(converted.headers[0].value, "Bearer a", "outro header fica");
        assert_eq!(converted.headers[1].value, BEARER_REFERENCE);
        assert!(!converted.headers[1].enabled, "continua desabilitado");
        assert_eq!(converted.headers[2].value, BEARER_REFERENCE);
    }

    #[test]
    fn different_tokens_in_one_request_are_rejected() {
        let req = request(vec![
            kv("Authorization", "Bearer a"),
            kv("Authorization", "Bearer b"),
        ]);
        assert_eq!(single_token(&req).unwrap_err().code(), "INVALID_REQUEST");
    }

    #[test]
    fn extract_creates_updates_and_keeps_secret() {
        let dir = workspace();
        env::create(dir.path(), "hml").unwrap();
        env::save_secrets(
            dir.path(),
            "hml",
            [
                ("outro".to_string(), Some("1".to_string())),
                (BEARER_SECRET_NAME.to_string(), None),
            ]
            .into(),
        )
        .unwrap();

        let mut req = request(vec![kv("Authorization", "Bearer t1")]);
        let out = extract(dir.path(), &mut req, Some("hml")).unwrap().unwrap();
        assert_eq!(
            out.action,
            SecretAction::Created,
            "segredo vazio conta como criado"
        );
        assert!(!out.gitignore_fixed);
        assert_eq!(req.headers[0].value, BEARER_REFERENCE);
        assert_eq!(
            env::reveal_secret(dir.path(), "hml", BEARER_SECRET_NAME).unwrap(),
            "t1"
        );
        assert_eq!(env::reveal_secret(dir.path(), "hml", "outro").unwrap(), "1");

        let mut same = request(vec![kv("Authorization", "Bearer t1")]);
        let out = extract(dir.path(), &mut same, Some("hml"))
            .unwrap()
            .unwrap();
        assert_eq!(out.action, SecretAction::Unchanged);

        let mut renewed = request(vec![kv("Authorization", "Bearer t2")]);
        assert_eq!(
            preview(dir.path(), &renewed, Some("hml")).unwrap(),
            Some(SecretAction::Updated)
        );
        let out = extract(dir.path(), &mut renewed, Some("hml"))
            .unwrap()
            .unwrap();
        assert_eq!(out.action, SecretAction::Updated);
        assert_eq!(
            env::reveal_secret(dir.path(), "hml", BEARER_SECRET_NAME).unwrap(),
            "t2"
        );
    }

    #[test]
    fn extract_without_literal_does_nothing() {
        let dir = workspace();
        let mut req = request(vec![kv("Authorization", BEARER_REFERENCE)]);
        assert_eq!(extract(dir.path(), &mut req, None).unwrap(), None);
        assert_eq!(preview(dir.path(), &req, None).unwrap(), None);
    }

    #[test]
    fn extract_needs_an_environment_and_writes_nothing_without_it() {
        let dir = workspace();
        let mut req = request(vec![kv("Authorization", "Bearer t")]);
        let err = extract(dir.path(), &mut req, None).unwrap_err();
        assert_eq!(err.code(), "BEARER_NEEDS_ENVIRONMENT");
        assert_eq!(req.headers[0].value, "Bearer t");
        let err = extract(dir.path(), &mut req, Some("nao-existe")).unwrap_err();
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn variable_with_the_same_name_blocks_extraction() {
        let dir = workspace();
        env::create(dir.path(), "hml").unwrap();
        // O ambiente novo já vem com o segredo vazio; aqui simula um ambiente antigo.
        std::fs::remove_file(dir.path().join("environments/hml.secrets.json")).unwrap();
        env::save_variables(
            dir.path(),
            "hml",
            [(BEARER_SECRET_NAME.to_string(), "x".to_string())].into(),
        )
        .unwrap();
        let mut req = request(vec![kv("Authorization", "Bearer t")]);
        assert_eq!(
            extract(dir.path(), &mut req, Some("hml"))
                .unwrap_err()
                .code(),
            "DUPLICATED_VARIABLE"
        );
    }

    #[test]
    fn extract_fixes_gitignore_before_writing_the_secret() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("environments")).unwrap();
        std::fs::write(
            dir.path().join("environments/hml.json"),
            r#"{"schemaVersion":1,"variables":{}}"#,
        )
        .unwrap();
        assert!(!gitignore::status(dir.path()));
        let mut req = request(vec![kv("Authorization", "Bearer t")]);
        let out = extract(dir.path(), &mut req, Some("hml")).unwrap().unwrap();
        assert!(out.gitignore_fixed);
        assert!(gitignore::status(dir.path()));
    }

    #[test]
    fn json_scan_finds_literal_authorization_only() {
        let literal = r#"{"schemaVersion":1,"headers":[{"key":"Authorization","value":"Bearer abc","enabled":false}]}"#;
        assert!(json_has_literal(literal));
        let reference =
            r#"{"headers":[{"key":"Authorization","value":"Bearer {{bearer_token}}"}]}"#;
        assert!(!json_has_literal(reference));
        let other = r#"{"headers":[{"key":"X-Note","value":"Bearer abc"}]}"#;
        assert!(!json_has_literal(other));
        assert!(!json_has_literal("{ inválido"));
    }

    #[test]
    fn outcome_serializes_in_camel_case() {
        let out = BearerOutcome {
            env: "hml".into(),
            action: SecretAction::Updated,
            gitignore_fixed: true,
        };
        assert_eq!(
            serde_json::to_value(out).unwrap(),
            serde_json::json!({ "env": "hml", "action": "updated", "gitignoreFixed": true })
        );
    }
}
