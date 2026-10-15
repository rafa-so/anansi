//! "Copiar como cURL" (RF-15, Seção 11.1).

use crate::env::resolver::{Report, Resolver};
use crate::env::EnvironmentData;
use crate::http::{automatic_content_type, encode_form};
use crate::model::{Body, HttpMethod, KeyValue, RequestFile};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CurlFormat {
    /// bash/POSIX: Git Bash, WSL, Linux, macOS.
    Bash,
    /// PowerShell 7.3+ (`curl.exe`, crase de continuação).
    Powershell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exported {
    pub curl: String,
    /// `true` quando algum segredo foi substituído pelo valor (aviso na interface).
    pub contains_secrets: bool,
}

/// Aspas simples para o formato escolhido.
fn quote(text: &str, format: CurlFormat) -> String {
    match format {
        CurlFormat::Bash => format!("'{}'", text.replace('\'', "'\\''")),
        CurlFormat::Powershell => format!("'{}'", text.replace('\'', "''")),
    }
}

/// Percent-encoding de componente de query, preservando `{` e `}` para os placeholders.
fn encode_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        let c = byte as char;
        if c.is_ascii_alphanumeric() || "-_.~{}".contains(c) {
            out.push(c);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn url_with_query(url: &str, query: &[KeyValue]) -> String {
    let pairs: Vec<String> = query
        .iter()
        .filter(|q| q.enabled && !(q.key.is_empty() && q.value.is_empty()))
        .map(|q| {
            format!(
                "{}={}",
                encode_component(&q.key),
                encode_component(&q.value)
            )
        })
        .collect();
    if pairs.is_empty() {
        return url.to_string();
    }
    let sep = if url.contains('?') { '&' } else { '?' };
    format!("{url}{sep}{}", pairs.join("&"))
}

/// Gera o comando. Com `resolve`, substitui variáveis e segredos do ambiente.
pub fn export(
    request: &RequestFile,
    env: Option<&EnvironmentData>,
    resolve: bool,
    format: CurlFormat,
) -> Exported {
    let mut report = Report::default();
    let source = if resolve {
        let (resolved, r) = Resolver::new(env).resolve_request(request);
        report = r;
        resolved
    } else {
        let mut copy = request.clone();
        copy.query.retain(|q| q.enabled);
        copy.headers.retain(|h| h.enabled);
        if let Body::FormUrlencoded { fields } = &mut copy.body {
            fields.retain(|f| f.enabled);
        }
        copy
    };

    let body: Option<String> = match &source.body {
        Body::None => None,
        Body::Json { content } | Body::Text { content } => Some(content.clone()),
        Body::FormUrlencoded { fields } => Some(encode_form(fields)),
    };

    let mut args: Vec<String> = Vec::new();
    let mut first = match format {
        CurlFormat::Bash => "curl".to_string(),
        CurlFormat::Powershell => "curl.exe".to_string(),
    };
    if !(source.method == HttpMethod::Get && body.is_none()) {
        first.push_str(&format!(" -X {}", source.method.as_str()));
    }
    first.push_str(&format!(
        " {}",
        quote(&url_with_query(&source.url, &source.query), format)
    ));

    let headers: Vec<&KeyValue> = source
        .headers
        .iter()
        .filter(|h| !h.key.trim().is_empty())
        .collect();
    for h in &headers {
        let line = if h.value.is_empty() {
            format!("{};", h.key)
        } else {
            format!("{}: {}", h.key, h.value)
        };
        args.push(format!("-H {}", quote(&line, format)));
    }
    if let Some(ct) = automatic_content_type(&source.body) {
        if !headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("content-type"))
        {
            args.push(format!(
                "-H {}",
                quote(&format!("Content-Type: {ct}"), format)
            ));
        }
    }
    if source.settings.verify_tls == Some(false) {
        args.push("-k".into());
    }
    if let Some(b) = body {
        args.push(format!("--data-raw {}", quote(&b, format)));
    }

    let continuation = match format {
        CurlFormat::Bash => " \\\n  ",
        CurlFormat::Powershell => " `\n  ",
    };
    let mut curl = first;
    for a in args {
        curl.push_str(continuation);
        curl.push_str(&a);
    }
    Exported {
        curl,
        contains_secrets: report.used_secret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::SecretValue;
    use crate::model::Settings;

    fn request() -> RequestFile {
        RequestFile {
            schema_version: 1,
            name: "Credencial".into(),
            method: HttpMethod::Post,
            url: "{{base_url_orders_api}}/orders".into(),
            query: vec![
                KeyValue {
                    key: "page".into(),
                    value: "1".into(),
                    enabled: true,
                },
                KeyValue {
                    key: "q".into(),
                    value: "a b".into(),
                    enabled: true,
                },
                KeyValue {
                    key: "off".into(),
                    value: "x".into(),
                    enabled: false,
                },
            ],
            headers: vec![
                KeyValue {
                    key: "x-api-key".into(),
                    value: "{{x-api-key}}".into(),
                    enabled: true,
                },
                KeyValue {
                    key: "X-Desligado".into(),
                    value: "1".into(),
                    enabled: false,
                },
            ],
            body: Body::Json {
                content: "{\n  \"Id\": \"12345678\",\n  \"nome\": \"D'Ávila\"\n}".into(),
            },
            settings: Settings::default(),
        }
    }

    fn env() -> EnvironmentData {
        EnvironmentData {
            variables: [(
                "base_url_orders_api".to_string(),
                "http://localhost:8080".to_string(),
            )]
            .into(),
            secrets: [("x-api-key".to_string(), SecretValue::new("s3cr3t-123"))].into(),
        }
    }

    #[test]
    fn bash_without_values() {
        let e = export(&request(), Some(&env()), false, CurlFormat::Bash);
        insta::assert_snapshot!("bash_sem_valores", e.curl);
        assert!(!e.contains_secrets);
    }

    #[test]
    fn bash_with_values_flags_secrets() {
        let e = export(&request(), Some(&env()), true, CurlFormat::Bash);
        insta::assert_snapshot!("bash_com_valores", e.curl);
        assert!(e.contains_secrets);
    }

    #[test]
    fn powershell_without_values() {
        let e = export(&request(), Some(&env()), false, CurlFormat::Powershell);
        insta::assert_snapshot!("powershell_sem_valores", e.curl);
    }

    #[test]
    fn get_without_body_omits_method_and_adds_insecure_flag() {
        let mut r = request();
        r.method = HttpMethod::Get;
        r.body = Body::None;
        r.query.clear();
        r.headers.clear();
        r.settings.verify_tls = Some(false);
        assert_eq!(
            export(&r, None, false, CurlFormat::Bash).curl,
            "curl '{{base_url_orders_api}}/orders' \\\n  -k"
        );
    }

    #[test]
    fn explicit_content_type_is_not_duplicated_and_empty_header_uses_semicolon() {
        let mut r = request();
        r.headers = vec![
            KeyValue {
                key: "content-type".into(),
                value: "application/vnd+json".into(),
                enabled: true,
            },
            KeyValue {
                key: "X-Vazio".into(),
                value: "".into(),
                enabled: true,
            },
        ];
        let curl = export(&r, None, false, CurlFormat::Bash).curl;
        assert_eq!(curl.matches("-H").count(), 2);
        assert!(curl.contains("-H 'X-Vazio;'"));
    }

    #[test]
    fn form_body_is_encoded() {
        let mut r = request();
        r.body = Body::FormUrlencoded {
            fields: vec![KeyValue {
                key: "a".into(),
                value: "1 2".into(),
                enabled: true,
            }],
        };
        let curl = export(&r, None, false, CurlFormat::Bash).curl;
        assert!(curl.contains("-H 'Content-Type: application/x-www-form-urlencoded'"));
        assert!(curl.ends_with("--data-raw 'a=1+2'"));
    }
}
