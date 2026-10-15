//! Leitura com checagem e migração de `schemaVersion` (RNF-06, Seção 4.6 da spec).

use crate::error::{AppError, AppResult};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Resultado da leitura de um arquivo versionado.
#[derive(Debug)]
pub struct Loaded<T> {
    pub value: T,
    /// Versão encontrada no arquivo, antes da migração.
    pub original_version: u32,
}

/// Lê um arquivo JSON versionado, aplicando as migrações em memória até `current`.
///
/// - JSON inválido ou sem `schemaVersion` → `INVALID_FILE`.
/// - Versão maior que `current` → `UNSUPPORTED_SCHEMA_VERSION` (o item fica somente leitura).
pub fn load<T: DeserializeOwned>(
    text: &str,
    current: u32,
    migrations: &[fn(Value) -> Value],
) -> AppResult<Loaded<T>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut value: Value = serde_json::from_str(text)
        .map_err(|e| AppError::InvalidFile(format!("JSON inválido: {e}")))?;
    let version = value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::InvalidFile("Campo schemaVersion ausente ou inválido.".into()))?
        as u32;
    if version == 0 {
        return Err(AppError::InvalidFile(
            "schemaVersion deve ser maior que zero.".into(),
        ));
    }
    if version > current {
        return Err(AppError::UnsupportedSchemaVersion(format!(
            "Criado por uma versão mais nova do anansi (schemaVersion {version}; esta versão lê até {current})."
        )));
    }
    // migrations[i] converte da versão i+1 para i+2.
    for migrate in migrations
        .iter()
        .skip(version as usize - 1)
        .take((current - version) as usize)
    {
        value = migrate(value);
    }
    if let Value::Object(map) = &mut value {
        map.insert("schemaVersion".into(), Value::from(current));
    }
    let parsed = serde_json::from_value(value)
        .map_err(|e| AppError::InvalidFile(format!("Formato incompatível: {e}")))?;
    Ok(Loaded {
        value: parsed,
        original_version: version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct Item {
        schema_version: u32,
        name: String,
    }

    #[test]
    fn reads_current_version_without_changes() {
        let loaded: Loaded<Item> = load(r#"{"schemaVersion":1,"name":"a"}"#, 1, &[]).unwrap();
        assert_eq!(
            loaded.value,
            Item {
                schema_version: 1,
                name: "a".into()
            }
        );
        assert_eq!(loaded.original_version, 1);
    }

    #[test]
    fn accepts_utf8_bom() {
        let loaded: Loaded<Item> =
            load("\u{feff}{\"schemaVersion\":1,\"name\":\"a\"}", 1, &[]).unwrap();
        assert_eq!(loaded.value.name, "a");
    }

    #[test]
    fn newer_version_is_unsupported() {
        let err = load::<Item>(r#"{"schemaVersion":2,"name":"a"}"#, 1, &[]).unwrap_err();
        assert_eq!(err.code(), "UNSUPPORTED_SCHEMA_VERSION");
        assert!(err.to_string().contains("versão mais nova"));
    }

    #[test]
    fn invalid_json_and_missing_version_are_invalid_file() {
        assert_eq!(
            load::<Item>("{", 1, &[]).unwrap_err().code(),
            "INVALID_FILE"
        );
        assert_eq!(
            load::<Item>(r#"{"name":"a"}"#, 1, &[]).unwrap_err().code(),
            "INVALID_FILE"
        );
        assert_eq!(
            load::<Item>(r#"{"schemaVersion":1}"#, 1, &[])
                .unwrap_err()
                .code(),
            "INVALID_FILE"
        );
    }

    #[test]
    fn applies_migrations_in_order() {
        fn v1_to_v2(mut v: Value) -> Value {
            let old = v["title"].take();
            v["name"] = old;
            v
        }
        fn v2_to_v3(mut v: Value) -> Value {
            let upper = v["name"].as_str().unwrap().to_uppercase();
            v["name"] = Value::from(upper);
            v
        }
        let loaded: Loaded<Item> = load(
            r#"{"schemaVersion":1,"title":"a"}"#,
            3,
            &[v1_to_v2, v2_to_v3],
        )
        .unwrap();
        assert_eq!(
            loaded.value,
            Item {
                schema_version: 3,
                name: "A".into()
            }
        );
        assert_eq!(loaded.original_version, 1);
        let from_v2: Loaded<Item> = load(
            r#"{"schemaVersion":2,"name":"b"}"#,
            3,
            &[v1_to_v2, v2_to_v3],
        )
        .unwrap();
        assert_eq!(from_v2.value.name, "B");
    }
}
