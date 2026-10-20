//! Request: formato em disco e formato trocado com o frontend (Seção 4.5 da spec).
//!
//! Em disco, os bodies `json` e `text` ficam num arquivo separado
//! (`<request>.body.json` / `.body.txt`, P-01). Para o frontend, o request é
//! um único objeto com `body.content`.

use serde::{Deserialize, Serialize};

pub const REQUEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Tipo de um campo do formulário: texto (padrão) ou arquivo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    #[default]
    Text,
    File,
}

impl FieldKind {
    pub fn is_text(&self) -> bool {
        matches!(self, FieldKind::Text)
    }
}

/// Campo do body de formulário. No tipo `file`, o `value` é o caminho do arquivo local.
///
/// O `type` só é gravado para arquivos, então os requests antigos continuam iguais.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(rename = "type", default, skip_serializing_if = "FieldKind::is_text")]
    pub kind: FieldKind,
}

impl FormField {
    pub fn text(key: &str, value: &str) -> Self {
        FormField {
            key: key.into(),
            value: value.into(),
            enabled: true,
            kind: FieldKind::Text,
        }
    }

    pub fn file(key: &str, path: &str) -> Self {
        FormField {
            kind: FieldKind::File,
            ..FormField::text(key, path)
        }
    }

    pub fn is_file(&self) -> bool {
        self.kind == FieldKind::File
    }
}

/// Configurações por request. `None` = usa o padrão das preferências globais.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify_tls: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_redirects: Option<bool>,
}

impl Settings {
    pub fn is_default(&self) -> bool {
        *self == Settings::default()
    }
}

/// Body como o frontend vê: com o conteúdo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Body {
    #[default]
    None,
    Json {
        #[serde(default)]
        content: String,
    },
    Text {
        #[serde(default)]
        content: String,
    },
    FormUrlencoded {
        #[serde(default)]
        fields: Vec<FormField>,
    },
}

impl Body {
    pub fn is_none(&self) -> bool {
        matches!(self, Body::None)
    }

    /// Formulário com algum campo de arquivo habilitado: é enviado como `multipart/form-data`.
    pub fn has_files(&self) -> bool {
        match self {
            Body::FormUrlencoded { fields } => fields.iter().any(|f| f.enabled && f.is_file()),
            _ => false,
        }
    }
}

/// Request completo trocado com o frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestFile {
    #[serde(default = "current_version")]
    pub schema_version: u32,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "Body::is_none")]
    pub body: Body,
    #[serde(default, skip_serializing_if = "Settings::is_default")]
    pub settings: Settings,
}

fn current_version() -> u32 {
    REQUEST_SCHEMA_VERSION
}

/// Body como fica gravado no arquivo do request (sem o conteúdo de json/text).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DiskBody {
    #[default]
    None,
    Json,
    Text,
    FormUrlencoded {
        #[serde(default)]
        fields: Vec<FormField>,
    },
}

impl DiskBody {
    pub fn is_none(&self) -> bool {
        matches!(self, DiskBody::None)
    }
}

/// Arquivo do request em disco. A ordem dos campos define a ordem no JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskRequest {
    pub schema_version: u32,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<KeyValue>,
    #[serde(default, skip_serializing_if = "DiskBody::is_none")]
    pub body: DiskBody,
    #[serde(default, skip_serializing_if = "Settings::is_default")]
    pub settings: Settings,
}

/// Arquivo de body que acompanha o request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyFileKind {
    Json,
    Text,
}

impl BodyFileKind {
    pub const ALL: [BodyFileKind; 2] = [BodyFileKind::Json, BodyFileKind::Text];

    /// Sufixo que substitui o `.json` do request: `x.json` → `x.body.json`.
    pub fn suffix(&self) -> &'static str {
        match self {
            BodyFileKind::Json => ".body.json",
            BodyFileKind::Text => ".body.txt",
        }
    }
}

/// Indica se um nome de arquivo é de body (e não de request).
pub fn is_body_file_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    BodyFileKind::ALL
        .iter()
        .any(|k| lower.ends_with(k.suffix()))
}

impl RequestFile {
    /// Separa o request no formato de disco e o conteúdo do body (se houver arquivo de body).
    pub fn split(&self) -> (DiskRequest, Option<(BodyFileKind, String)>) {
        let (body, file) = match &self.body {
            Body::None => (DiskBody::None, None),
            Body::Json { content } => (
                DiskBody::Json,
                Some((BodyFileKind::Json, normalize_eol(content))),
            ),
            Body::Text { content } => (
                DiskBody::Text,
                Some((BodyFileKind::Text, normalize_eol(content))),
            ),
            Body::FormUrlencoded { fields } => (
                DiskBody::FormUrlencoded {
                    fields: fields.clone(),
                },
                None,
            ),
        };
        let disk = DiskRequest {
            schema_version: REQUEST_SCHEMA_VERSION,
            name: self.name.clone(),
            method: self.method,
            url: self.url.clone(),
            query: self.query.clone(),
            headers: self.headers.clone(),
            body,
            settings: self.settings.clone(),
        };
        (disk, file)
    }

    /// Junta o request lido do disco com o conteúdo do arquivo de body.
    pub fn join(disk: DiskRequest, body_content: Option<String>) -> RequestFile {
        let content = body_content.unwrap_or_default();
        let body = match disk.body {
            DiskBody::None => Body::None,
            DiskBody::Json => Body::Json { content },
            DiskBody::Text => Body::Text { content },
            DiskBody::FormUrlencoded { fields } => Body::FormUrlencoded { fields },
        };
        RequestFile {
            schema_version: disk.schema_version,
            name: disk.name,
            method: disk.method,
            url: disk.url,
            query: disk.query,
            headers: disk.headers,
            body,
            settings: disk.settings,
        }
    }
}

/// Converte `\r\n` (e `\r` solto) em `\n`.
pub fn normalize_eol(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn sample(body: Body) -> RequestFile {
        RequestFile {
            schema_version: 1,
            name: "Criar produto".into(),
            method: HttpMethod::Post,
            url: "{{baseUrlStore}}/products".into(),
            query: vec![],
            headers: vec![KeyValue {
                key: "Authorization".into(),
                value: "Bearer {{token}}".into(),
                enabled: true,
            }],
            body,
            settings: Settings::default(),
        }
    }

    #[test]
    fn json_body_goes_to_separate_file() {
        let req = sample(Body::Json {
            content: "{\r\n  \"id\": {{produtoId}}\r\n}".into(),
        });
        let (disk, file) = req.split();
        assert_eq!(disk.body, DiskBody::Json);
        assert_eq!(
            file,
            Some((
                BodyFileKind::Json,
                "{\n  \"id\": {{produtoId}}\n}".to_string()
            ))
        );
    }

    #[test]
    fn form_body_stays_inline() {
        let fields = vec![
            FormField::text("a", "1"),
            FormField::file("anexo", "C:/docs/a.pdf"),
        ];
        let (disk, file) = sample(Body::FormUrlencoded {
            fields: fields.clone(),
        })
        .split();
        assert_eq!(disk.body, DiskBody::FormUrlencoded { fields });
        assert!(file.is_none());
    }

    #[test]
    fn split_then_join_round_trips() {
        for body in [
            Body::None,
            Body::Json {
                content: "{}".into(),
            },
            Body::Text {
                content: "texto".into(),
            },
            Body::FormUrlencoded { fields: vec![] },
        ] {
            let req = sample(body);
            let (disk, file) = req.split();
            assert_eq!(RequestFile::join(disk, file.map(|(_, c)| c)), req);
        }
    }

    #[test]
    fn missing_body_file_reads_as_empty() {
        let (disk, _) = sample(Body::Text {
            content: "x".into(),
        })
        .split();
        assert_eq!(
            RequestFile::join(disk, None).body,
            Body::Text {
                content: String::new()
            }
        );
    }

    #[test]
    fn frontend_format_uses_content_and_kebab_case() {
        let json = serde_json::to_value(sample(Body::FormUrlencoded { fields: vec![] })).unwrap();
        assert_eq!(
            json["body"],
            serde_json::json!({ "type": "form-urlencoded", "fields": [] })
        );
        assert_eq!(json["schemaVersion"], 1);
        assert!(json.get("query").is_none(), "lista vazia é omitida");
        assert!(json.get("settings").is_none(), "settings padrão é omitido");
    }

    #[test]
    fn form_field_type_is_only_written_for_files() {
        let json = serde_json::to_value(vec![
            FormField::text("a", "1"),
            FormField::file("b", "/tmp/x.png"),
        ])
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                { "key": "a", "value": "1", "enabled": true },
                { "key": "b", "value": "/tmp/x.png", "enabled": true, "type": "file" }
            ])
        );
        let old: FormField = serde_json::from_str(r#"{"key":"a","value":"1"}"#).unwrap();
        assert_eq!(
            old,
            FormField::text("a", "1"),
            "campo antigo, sem type, é texto"
        );
    }

    #[test]
    fn has_files_only_counts_enabled_file_fields() {
        let mut off = FormField::file("b", "x");
        off.enabled = false;
        assert!(!Body::FormUrlencoded {
            fields: vec![FormField::text("a", "1"), off]
        }
        .has_files());
        assert!(Body::FormUrlencoded {
            fields: vec![FormField::file("b", "x")]
        }
        .has_files());
        assert!(!Body::None.has_files());
    }

    #[test]
    fn body_file_names() {
        assert!(is_body_file_name("criar.body.json"));
        assert!(is_body_file_name("Criar.BODY.txt"));
        assert!(!is_body_file_name("criar.json"));
        assert!(!is_body_file_name("body.json"));
    }
}
