//! Ambientes, variáveis e segredos (Seções 4.5, 6 e 7 da spec).
//!
//! - `environments/<nome>.json`: variáveis versionadas.
//! - `environments/<nome>.secrets.json`: segredos locais, ignorados pelo git.
//!
//! Valores de segredos ficam em `SecretValue`, que nunca aparece em logs nem
//! em mensagens de erro, e só saem do backend por `reveal_secret`.

pub mod gitignore;
pub mod resolver;

use crate::error::{AppError, AppResult};
use crate::model::schema::load;
use crate::model::to_file_bytes;
use crate::workspace::fs_ops;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub const ENVIRONMENTS_DIR: &str = "environments";
pub const ENV_SCHEMA_VERSION: u32 = 1;

/// Valor de segredo. `Debug` e `Display` imprimem `***` (Seção 7.2).
#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue(String);

impl SecretValue {
    pub fn new(value: impl Into<String>) -> Self {
        SecretValue(value.into())
    }

    /// Acesso explícito ao valor real.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnvFile {
    schema_version: u32,
    #[serde(default)]
    variables: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecretsFile {
    schema_version: u32,
    #[serde(default)]
    secrets: BTreeMap<String, String>,
}

/// Variáveis e segredos carregados de um ambiente.
#[derive(Debug, Clone, Default)]
pub struct EnvironmentData {
    pub variables: BTreeMap<String, String>,
    pub secrets: BTreeMap<String, SecretValue>,
}

impl EnvironmentData {
    /// Nomes definidos com o mesmo nome como variável e como segredo (edição manual).
    pub fn duplicated(&self) -> Vec<String> {
        self.variables
            .keys()
            .filter(|k| self.secrets.contains_key(*k))
            .cloned()
            .collect()
    }

    /// Todos os nomes conhecidos (variáveis e segredos), sem repetição e em ordem.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .variables
            .keys()
            .chain(self.secrets.keys())
            .cloned()
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

/// O que o frontend vê de um ambiente: valores das variáveis e só os nomes dos segredos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvView {
    pub name: String,
    pub variables: BTreeMap<String, String>,
    pub secret_names: Vec<String>,
    pub warnings: Vec<String>,
}

/// Lista os ambientes (`environments/<nome>.json`, exceto `*.secrets.json`), em ordem alfabética.
pub fn list(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join(ENVIRONMENTS_DIR)) else {
        return vec![];
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".json")?;
            if stem.ends_with(".secrets") || !is_valid_env_name(stem) {
                return None;
            }
            Some(stem.to_string())
        })
        .collect();
    names.sort();
    names
}

/// Nome de ambiente válido: `[a-z0-9-]+` (casa com o padrão do `.gitignore`).
pub fn is_valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Nome de variável ou segredo válido: `[A-Za-z0-9_.-]+` (RN-07).
pub fn is_valid_var_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}

fn env_path(root: &Path, name: &str) -> PathBuf {
    root.join(ENVIRONMENTS_DIR).join(format!("{name}.json"))
}

fn secrets_path(root: &Path, name: &str) -> PathBuf {
    root.join(ENVIRONMENTS_DIR)
        .join(format!("{name}.secrets.json"))
}

fn check_env_name(name: &str) -> AppResult<()> {
    if !is_valid_env_name(name) {
        return Err(AppError::InvalidName(format!(
            "Nome de ambiente inválido: \"{name}\". Use só letras minúsculas, números e hífen."
        )));
    }
    Ok(())
}

fn check_var_names<'a>(names: impl Iterator<Item = &'a String>) -> AppResult<()> {
    for name in names {
        if !is_valid_var_name(name) {
            return Err(AppError::InvalidName(format!(
                "Nome inválido: \"{name}\". Use letras, números, \"_\", \".\" e \"-\", sem espaços."
            )));
        }
    }
    Ok(())
}

fn require_gitignore(root: &Path) -> AppResult<()> {
    if !gitignore::status(root) {
        return Err(AppError::GitignoreNotConfigured(
            "O .gitignore não ignora environments/*.secrets.json. Use \"Corrigir .gitignore\" antes de mexer em segredos.".into(),
        ));
    }
    Ok(())
}

fn read_variables(root: &Path, name: &str) -> AppResult<BTreeMap<String, String>> {
    let path = env_path(root, name);
    let text = std::fs::read_to_string(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            AppError::NotFound(format!("Ambiente não encontrado: {name}"))
        }
        _ => AppError::io(&e, &path),
    })?;
    load::<EnvFile>(&text, ENV_SCHEMA_VERSION, &[])
        .map(|l| l.value.variables)
        .map_err(|e| AppError::InvalidFile(format!("environments/{name}.json: {e}")))
}

fn read_secrets(root: &Path, name: &str) -> AppResult<BTreeMap<String, SecretValue>> {
    let path = secrets_path(root, name);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(AppError::io(&e, &path)),
    };
    let file = load::<SecretsFile>(&text, ENV_SCHEMA_VERSION, &[])
        .map_err(|e| AppError::InvalidFile(format!("environments/{name}.secrets.json: {e}")))?;
    Ok(file
        .value
        .secrets
        .into_iter()
        .map(|(k, v)| (k, SecretValue(v)))
        .collect())
}

/// Carrega variáveis e segredos (RN-08: o arquivo de segredos é opcional).
pub fn load_environment(root: &Path, name: &str) -> AppResult<EnvironmentData> {
    check_env_name(name)?;
    Ok(EnvironmentData {
        variables: read_variables(root, name)?,
        secrets: read_secrets(root, name)?,
    })
}

/// Visão do ambiente para a tela de ambientes: nunca inclui valores de segredos.
pub fn view(root: &Path, name: &str) -> AppResult<EnvView> {
    let data = load_environment(root, name)?;
    let warnings = data
        .duplicated()
        .into_iter()
        .map(|n| format!("\"{n}\" está definido como variável e como segredo; vale o segredo."))
        .collect();
    Ok(EnvView {
        name: name.to_string(),
        variables: data.variables,
        secret_names: data.secrets.keys().cloned().collect(),
        warnings,
    })
}

pub fn create(root: &Path, name: &str) -> AppResult<()> {
    check_env_name(name)?;
    let path = env_path(root, name);
    if path.exists() {
        return Err(AppError::AlreadyExists(format!(
            "Já existe o ambiente \"{name}\"."
        )));
    }
    std::fs::create_dir_all(root.join(ENVIRONMENTS_DIR)).map_err(|e| AppError::io(&e, root))?;
    let file = EnvFile {
        schema_version: ENV_SCHEMA_VERSION,
        variables: BTreeMap::new(),
    };
    fs_ops::write_atomic(&path, &to_file_bytes(&file)).map(|_| ())
}

/// Remove o ambiente e o arquivo de segredos dele.
pub fn delete(root: &Path, name: &str) -> AppResult<()> {
    check_env_name(name)?;
    let path = env_path(root, name);
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Ambiente não encontrado: {name}"
        )));
    }
    fs_ops::remove_file_if_exists(&secrets_path(root, name))?;
    fs_ops::remove_file_if_exists(&path)
}

/// Grava as variáveis (substitui o conjunto inteiro).
pub fn save_variables(
    root: &Path,
    name: &str,
    variables: BTreeMap<String, String>,
) -> AppResult<()> {
    check_env_name(name)?;
    read_variables(root, name)?;
    check_var_names(variables.keys())?;
    let secrets = read_secrets(root, name)?;
    if let Some(dup) = variables.keys().find(|k| secrets.contains_key(*k)) {
        return Err(AppError::DuplicatedVariable(format!(
            "\"{dup}\" já é um segredo neste ambiente. Variáveis e segredos precisam ter nomes diferentes."
        )));
    }
    let file = EnvFile {
        schema_version: ENV_SCHEMA_VERSION,
        variables,
    };
    fs_ops::write_atomic(&env_path(root, name), &to_file_bytes(&file)).map(|_| ())
}

/// Grava os segredos. O mapa é o conjunto final: chaves ausentes são removidas e
/// `None` mantém o valor atual (o frontend não precisa reenviar o que não mudou).
pub fn save_secrets(
    root: &Path,
    name: &str,
    secrets: BTreeMap<String, Option<String>>,
) -> AppResult<()> {
    check_env_name(name)?;
    require_gitignore(root)?;
    let variables = read_variables(root, name)?;
    check_var_names(secrets.keys())?;
    if let Some(dup) = secrets.keys().find(|k| variables.contains_key(*k)) {
        return Err(AppError::DuplicatedVariable(format!(
            "\"{dup}\" já é uma variável neste ambiente. Variáveis e segredos precisam ter nomes diferentes."
        )));
    }
    let current = read_secrets(root, name)?;
    let mut result = BTreeMap::new();
    for (key, value) in secrets {
        let value = match value {
            Some(v) => v,
            None => match current.get(&key) {
                Some(v) => v.expose().to_string(),
                None => continue,
            },
        };
        result.insert(key, value);
    }
    let path = secrets_path(root, name);
    if result.is_empty() && !path.exists() {
        return Ok(());
    }
    let file = SecretsFile {
        schema_version: ENV_SCHEMA_VERSION,
        secrets: result,
    };
    fs_ops::write_atomic(&path, &to_file_bytes(&file)).map(|_| ())
}

/// Devolve o valor de um segredo (botão "mostrar", RF-13).
pub fn reveal_secret(root: &Path, name: &str, key: &str) -> AppResult<String> {
    check_env_name(name)?;
    require_gitignore(root)?;
    read_secrets(root, name)?
        .get(key)
        .map(|v| v.expose().to_string())
        .ok_or_else(|| AppError::NotFound(format!("Segredo não encontrado: {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn workspace(gitignore_ok: bool) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(ENVIRONMENTS_DIR)).unwrap();
        if gitignore_ok {
            std::fs::write(
                dir.path().join(".gitignore"),
                "environments/*.secrets.json\n",
            )
            .unwrap();
        }
        dir
    }

    fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn lists_environments_without_secrets() {
        let dir = workspace(true);
        let envs = dir.path().join(ENVIRONMENTS_DIR);
        for f in [
            "prd.json",
            "hml.json",
            "hml.secrets.json",
            "local.json",
            ".gitkeep",
            "Invalido.json",
            "notas.txt",
        ] {
            std::fs::write(envs.join(f), "{}").unwrap();
        }
        assert_eq!(list(dir.path()), vec!["hml", "local", "prd"]);
    }

    #[test]
    fn no_environments_folder_means_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list(dir.path()).is_empty());
    }

    #[test]
    fn secret_value_never_prints() {
        let s = SecretValue::new("eyJhbGciOi");
        assert_eq!(format!("{s:?} {s}"), "*** ***");
        assert_eq!(s.expose(), "eyJhbGciOi");
        let data = EnvironmentData {
            variables: BTreeMap::new(),
            secrets: [("token".to_string(), s)].into(),
        };
        assert!(!format!("{data:?}").contains("eyJ"));
    }

    #[test]
    fn create_save_and_view_environment() {
        let dir = workspace(true);
        create(dir.path(), "hml").unwrap();
        assert_eq!(
            create(dir.path(), "hml").unwrap_err().code(),
            "ALREADY_EXISTS"
        );
        assert_eq!(
            create(dir.path(), "HML").unwrap_err().code(),
            "INVALID_NAME"
        );
        save_variables(
            dir.path(),
            "hml",
            vars(&[("baseUrl", "https://x"), ("a.b-c_d", "1")]),
        )
        .unwrap();
        save_secrets(
            dir.path(),
            "hml",
            [("token".to_string(), Some("segredo".to_string()))].into(),
        )
        .unwrap();
        let view = view(dir.path(), "hml").unwrap();
        assert_eq!(
            view.variables,
            vars(&[("a.b-c_d", "1"), ("baseUrl", "https://x")])
        );
        assert_eq!(view.secret_names, vec!["token"]);
        assert!(
            !serde_json::to_string(&view).unwrap().contains("segredo"),
            "valores de segredos nunca vão na visão"
        );
        let text = std::fs::read_to_string(dir.path().join("environments/hml.json")).unwrap();
        assert_eq!(
            text,
            "{\n  \"schemaVersion\": 1,\n  \"variables\": {\n    \"a.b-c_d\": \"1\",\n    \"baseUrl\": \"https://x\"\n  }\n}\n"
        );
    }

    #[test]
    fn variable_and_secret_names_must_be_unique() {
        let dir = workspace(true);
        create(dir.path(), "hml").unwrap();
        save_secrets(
            dir.path(),
            "hml",
            [("token".to_string(), Some("x".to_string()))].into(),
        )
        .unwrap();
        assert_eq!(
            save_variables(dir.path(), "hml", vars(&[("token", "y")]))
                .unwrap_err()
                .code(),
            "DUPLICATED_VARIABLE"
        );
        save_variables(dir.path(), "hml", vars(&[("baseUrl", "y")])).unwrap();
        let err = save_secrets(
            dir.path(),
            "hml",
            [("baseUrl".to_string(), Some("z".to_string()))].into(),
        )
        .unwrap_err();
        assert_eq!(err.code(), "DUPLICATED_VARIABLE");
    }

    #[test]
    fn invalid_variable_names_are_rejected() {
        let dir = workspace(true);
        create(dir.path(), "local").unwrap();
        assert_eq!(
            save_variables(dir.path(), "local", vars(&[("com espaço", "1")]))
                .unwrap_err()
                .code(),
            "INVALID_NAME"
        );
        assert_eq!(
            save_variables(dir.path(), "local", vars(&[("", "1")]))
                .unwrap_err()
                .code(),
            "INVALID_NAME"
        );
    }

    #[test]
    fn null_keeps_secret_and_missing_key_removes_it() {
        let dir = workspace(true);
        create(dir.path(), "hml").unwrap();
        save_secrets(
            dir.path(),
            "hml",
            [
                ("a".into(), Some("1".into())),
                ("b".into(), Some("2".into())),
            ]
            .into(),
        )
        .unwrap();
        save_secrets(
            dir.path(),
            "hml",
            [("a".into(), None), ("c".into(), Some("3".into()))].into(),
        )
        .unwrap();
        assert_eq!(reveal_secret(dir.path(), "hml", "a").unwrap(), "1");
        assert_eq!(
            reveal_secret(dir.path(), "hml", "b").unwrap_err().code(),
            "NOT_FOUND"
        );
        assert_eq!(reveal_secret(dir.path(), "hml", "c").unwrap(), "3");
    }

    #[test]
    fn secrets_require_gitignore() {
        let dir = workspace(false);
        create(dir.path(), "hml").unwrap();
        let err =
            save_secrets(dir.path(), "hml", [("t".into(), Some("x".into()))].into()).unwrap_err();
        assert_eq!(err.code(), "GITIGNORE_NOT_CONFIGURED");
        assert_eq!(
            reveal_secret(dir.path(), "hml", "t").unwrap_err().code(),
            "GITIGNORE_NOT_CONFIGURED"
        );
        assert!(!dir.path().join("environments/hml.secrets.json").exists());
        // As variáveis continuam editáveis.
        save_variables(dir.path(), "hml", vars(&[("a", "1")])).unwrap();
    }

    #[test]
    fn hand_edited_duplicate_is_reported_and_secret_wins() {
        let dir = workspace(true);
        std::fs::write(
            dir.path().join("environments/hml.json"),
            r#"{"schemaVersion":1,"variables":{"token":"var"}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("environments/hml.secrets.json"),
            r#"{"schemaVersion":1,"secrets":{"token":"sec"}}"#,
        )
        .unwrap();
        let v = view(dir.path(), "hml").unwrap();
        assert_eq!(v.warnings.len(), 1);
        let data = load_environment(dir.path(), "hml").unwrap();
        assert_eq!(data.duplicated(), vec!["token"]);
        assert_eq!(data.names(), vec!["token"]);
    }

    #[test]
    fn delete_removes_both_files() {
        let dir = workspace(true);
        create(dir.path(), "hml").unwrap();
        save_secrets(dir.path(), "hml", [("t".into(), Some("x".into()))].into()).unwrap();
        delete(dir.path(), "hml").unwrap();
        assert!(list(dir.path()).is_empty());
        assert!(!dir.path().join("environments/hml.secrets.json").exists());
        assert_eq!(delete(dir.path(), "hml").unwrap_err().code(), "NOT_FOUND");
    }

    #[test]
    fn missing_environment_and_invalid_files() {
        let dir = workspace(true);
        assert_eq!(
            load_environment(dir.path(), "nada").unwrap_err().code(),
            "NOT_FOUND"
        );
        std::fs::write(dir.path().join("environments/x.json"), "{").unwrap();
        assert_eq!(
            load_environment(dir.path(), "x").unwrap_err().code(),
            "INVALID_FILE"
        );
        assert_eq!(
            load_environment(dir.path(), "../x").unwrap_err().code(),
            "INVALID_NAME"
        );
    }
}
