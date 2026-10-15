//! "Colar cURL" (RF-16, Seção 11.2): só o formato bash/POSIX.

use crate::error::{AppError, AppResult};
use crate::model::{Body, HttpMethod, KeyValue, RequestFile, Settings, REQUEST_SCHEMA_VERSION};
use base64::Engine;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub request: RequestFile,
    pub warnings: Vec<String>,
}

/// Flags que não mudam a definição do request: ignoradas sem aviso.
const IGNORED: &[&str] = &[
    "-L",
    "--location",
    "--compressed",
    "-s",
    "--silent",
    "-S",
    "--show-error",
    "-v",
    "--verbose",
    "-i",
    "--include",
    "-g",
    "--globoff",
    "--location-trusted",
    "-#",
    "--progress-bar",
    "-f",
    "--fail",
    "--http1.1",
    "--http2",
];

/// Flags desconhecidas que recebem valor: o valor é pulado junto (com aviso).
const UNKNOWN_WITH_VALUE: &[&str] = &[
    "-o",
    "--output",
    "-w",
    "--write-out",
    "--connect-timeout",
    "--retry",
    "-x",
    "--proxy",
    "--cacert",
    "--cert",
    "--key",
    "-c",
    "--cookie-jar",
    "-F",
    "--form",
    "-T",
    "--upload-file",
    "--resolve",
    "-r",
    "--range",
];

fn invalid(msg: &str) -> AppError {
    AppError::InvalidFile(msg.to_string())
}

/// Junta continuações de linha e tokeniza como o shell faria.
fn tokenize(text: &str) -> AppResult<Vec<String>> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() > 1 && lines.iter().take(lines.len() - 1).all(|l| l.ends_with('^')) {
        return Err(invalid(
            "Este é o formato \"Copy as cURL (cmd)\". Copie de novo usando \"Copy as cURL (bash)\", que o Chrome e o Edge também oferecem.",
        ));
    }
    let joined = text.replace("\\\n", " ");
    shell_words::split(&joined)
        .map_err(|_| invalid("Não foi possível ler o comando: há aspas sem fechar?"))
}

/// Separa a query string da URL (a tabela de Params é a fonte da verdade).
fn split_query(url: &str) -> (String, Vec<KeyValue>) {
    let without_hash = url.split('#').next().unwrap_or(url);
    match without_hash.split_once('?') {
        None => (without_hash.to_string(), vec![]),
        Some((base, query)) => {
            let pairs = url::form_urlencoded::parse(query.as_bytes())
                .map(|(k, v)| KeyValue {
                    key: k.into_owned(),
                    value: v.into_owned(),
                    enabled: true,
                })
                .collect();
            (base.to_string(), pairs)
        }
    }
}

fn parse_header(raw: &str) -> Option<KeyValue> {
    // `-H 'nome;'` envia o header vazio (semântica do curl).
    if let Some(name) = raw.strip_suffix(';') {
        if !name.contains(':') {
            return Some(KeyValue {
                key: name.trim().to_string(),
                value: String::new(),
                enabled: true,
            });
        }
    }
    let (k, v) = raw.split_once(':')?;
    Some(KeyValue {
        key: k.trim().to_string(),
        value: v.trim_start().to_string(),
        enabled: true,
    })
}

pub fn import(text: &str) -> AppResult<Imported> {
    let tokens = tokenize(text)?;
    let mut iter = tokens.into_iter().peekable();
    match iter.next().as_deref() {
        Some("curl") | Some("curl.exe") => {}
        _ => {
            return Err(invalid(
                "O texto não parece um comando cURL (precisa começar com \"curl\").",
            ))
        }
    }

    let mut url: Option<String> = None;
    let mut method: Option<String> = None;
    let mut headers: Vec<KeyValue> = vec![];
    let mut data: Vec<String> = vec![];
    let mut settings = Settings::default();
    let mut warnings: Vec<String> = vec![];

    while let Some(token) = iter.next() {
        // `--flag=valor` e `-XPOST` / `-HAccept: x`
        let (flag, inline): (String, Option<String>) = if token.starts_with("--") {
            match token.split_once('=') {
                Some((f, v)) => (f.to_string(), Some(v.to_string())),
                None => (token.clone(), None),
            }
        } else if token.starts_with('-')
            && token.len() > 2
            && ["-X", "-H", "-d", "-u", "-b", "-m", "-A", "-e"].contains(&&token[..2])
        {
            (token[..2].to_string(), Some(token[2..].to_string()))
        } else {
            (token.clone(), None)
        };
        let mut value = |name: &str| -> AppResult<String> {
            match inline.clone() {
                Some(v) => Ok(v),
                None => iter
                    .next()
                    .ok_or_else(|| invalid(&format!("A opção {name} está sem valor."))),
            }
        };
        match flag.as_str() {
            "-X" | "--request" => method = Some(value(&flag)?.to_uppercase()),
            "-H" | "--header" => {
                let raw = value(&flag)?;
                match parse_header(&raw) {
                    Some(h) => headers.push(h),
                    None => warnings.push(format!("Header ignorado (sem \":\"): {raw}")),
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii"
            | "--data-urlencode" => {
                let v = value(&flag)?;
                if v.starts_with('@') && flag != "--data-raw" {
                    warnings.push(format!("Body lido de arquivo ({v}) não é suportado; o texto foi mantido como está."));
                }
                if flag == "--data-urlencode" {
                    warnings
                        .push("--data-urlencode foi tratado como --data (sem codificar).".into());
                }
                data.push(v);
            }
            "-u" | "--user" => {
                let creds = value(&flag)?;
                let encoded = base64::engine::general_purpose::STANDARD.encode(creds.as_bytes());
                headers.push(KeyValue {
                    key: "Authorization".into(),
                    value: format!("Basic {encoded}"),
                    enabled: true,
                });
            }
            "-b" | "--cookie" => headers.push(KeyValue {
                key: "Cookie".into(),
                value: value(&flag)?,
                enabled: true,
            }),
            "-A" | "--user-agent" => headers.push(KeyValue {
                key: "User-Agent".into(),
                value: value(&flag)?,
                enabled: true,
            }),
            "-e" | "--referer" => headers.push(KeyValue {
                key: "Referer".into(),
                value: value(&flag)?,
                enabled: true,
            }),
            "-k" | "--insecure" => settings.verify_tls = Some(false),
            "-m" | "--max-time" => {
                let v = value(&flag)?;
                match v.parse::<f64>() {
                    Ok(secs) if secs > 0.0 => settings.timeout_ms = Some((secs * 1000.0) as u64),
                    _ => warnings.push(format!("{flag} com valor inválido ignorado: {v}")),
                }
            }
            "--url" => url = Some(value(&flag)?),
            f if IGNORED.contains(&f) => {}
            f if UNKNOWN_WITH_VALUE.contains(&f) => {
                let v = value(f)?;
                warnings.push(format!("Opção ignorada: {f} {v}"));
            }
            f if f.starts_with('-') && f.len() > 1 => warnings.push(format!("Opção ignorada: {f}")),
            _ => {
                if url.is_some() {
                    warnings.push(format!("Argumento ignorado: {token}"));
                } else {
                    url = Some(token);
                }
            }
        }
    }

    let url = url.ok_or_else(|| invalid("O comando cURL não tem URL."))?;
    let (base_url, query) = split_query(&url);

    let has_data = !data.is_empty();
    let method_text = method.unwrap_or_else(|| {
        if has_data {
            "POST".into()
        } else {
            "GET".into()
        }
    });
    let method: HttpMethod =
        serde_json::from_value(serde_json::Value::from(method_text.clone()))
            .map_err(|_| invalid(&format!("Método HTTP não suportado: {method_text}")))?;

    let content_type = headers
        .iter()
        .find(|h| h.key.eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.to_ascii_lowercase());
    let joined = data.join("&");
    let body = if !has_data {
        Body::None
    } else {
        match content_type.as_deref() {
            Some(ct) if ct.contains("json") => Body::Json { content: joined },
            Some(ct) if ct.contains("application/x-www-form-urlencoded") => form_body(&joined),
            Some(_) => Body::Text { content: joined },
            // Sem Content-Type, o curl envia -d como form-urlencoded.
            None => form_body(&joined),
        }
    };

    let last_segment = base_url
        .split('?')
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty() && !s.contains("://"))
        .unwrap_or("request")
        .to_string();

    Ok(Imported {
        request: RequestFile {
            schema_version: REQUEST_SCHEMA_VERSION,
            name: format!("{} {}", method.as_str(), last_segment),
            method,
            url: base_url,
            query,
            headers,
            body,
            settings,
        },
        warnings,
    })
}

fn form_body(text: &str) -> Body {
    let fields = url::form_urlencoded::parse(text.as_bytes())
        .map(|(k, v)| KeyValue {
            key: k.into_owned(),
            value: v.into_owned(),
            enabled: true,
        })
        .collect();
    Body::FormUrlencoded { fields }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curl::export::{export, CurlFormat};
    use pretty_assertions::assert_eq;

    fn kv(k: &str, v: &str) -> KeyValue {
        KeyValue {
            key: k.into(),
            value: v.into(),
            enabled: true,
        }
    }

    /// O cURL do Postman que motivou a P-02 (com --location, --globoff e header vazio).
    #[test]
    fn imports_postman_export() {
        let text = "curl --location --globoff '{{base_url_orders_api}}/orders' \\\n--header 'x-api-key;' \\\n--header 'Content-Type: application/json' \\\n--data '{\n    \"Id\": \"12345678\",\n    \"isAdult\": true\n}'";
        let imported = import(text).unwrap();
        assert_eq!(imported.warnings, Vec::<String>::new());
        let r = imported.request;
        assert_eq!(r.method, HttpMethod::Post);
        assert_eq!(r.name, "POST orders");
        assert_eq!(r.url, "{{base_url_orders_api}}/orders");
        assert_eq!(
            r.headers,
            vec![
                kv("x-api-key", ""),
                kv("Content-Type", "application/json")
            ]
        );
        assert_eq!(
            r.body,
            Body::Json {
                content: "{\n    \"Id\": \"12345678\",\n    \"isAdult\": true\n}".into()
            }
        );
    }

    #[test]
    fn imports_chrome_bash_copy_with_crlf() {
        let text = "curl 'https://api.exemplo.com/products?page=1&q=a%20b' \\\r\n  -H 'accept: application/json' \\\r\n  -H 'authorization: Bearer abc' \\\r\n  -b 'sid=1; x=2' \\\r\n  --compressed";
        let r = import(text).unwrap().request;
        assert_eq!(r.method, HttpMethod::Get);
        assert_eq!(r.url, "https://api.exemplo.com/products");
        assert_eq!(r.query, vec![kv("page", "1"), kv("q", "a b")]);
        assert_eq!(r.headers.len(), 3);
        assert_eq!(r.headers[2], kv("Cookie", "sid=1; x=2"));
        assert_eq!(r.name, "GET products");
    }

    #[test]
    fn data_without_content_type_is_form_and_multiple_data_are_joined() {
        let r = import("curl https://x/login -d user=a -d 'pass=b c'")
            .unwrap()
            .request;
        assert_eq!(r.method, HttpMethod::Post);
        assert_eq!(
            r.body,
            Body::FormUrlencoded {
                fields: vec![kv("user", "a"), kv("pass", "b c")]
            }
        );
    }

    #[test]
    fn explicit_method_basic_auth_insecure_and_timeout() {
        let r = import("curl -XPUT -u admin:senha -k -m 2.5 --url https://x/a -H 'Content-Type: text/plain' --data-raw 'oi'").unwrap().request;
        assert_eq!(r.method, HttpMethod::Put);
        assert_eq!(r.headers[0], kv("Authorization", "Basic YWRtaW46c2VuaGE="));
        assert_eq!(r.settings.verify_tls, Some(false));
        assert_eq!(r.settings.timeout_ms, Some(2500));
        assert_eq!(
            r.body,
            Body::Text {
                content: "oi".into()
            }
        );
    }

    #[test]
    fn unknown_flags_are_listed() {
        let imported = import("curl -o saida.json --http3 https://x/a").unwrap();
        assert_eq!(
            imported.warnings,
            vec!["Opção ignorada: -o saida.json", "Opção ignorada: --http3"]
        );
        assert_eq!(imported.request.url, "https://x/a");
    }

    #[test]
    fn errors_are_clear() {
        assert!(import("wget https://x")
            .unwrap_err()
            .to_string()
            .contains("não parece um comando cURL"));
        assert!(import("curl -H 'a: b'")
            .unwrap_err()
            .to_string()
            .contains("não tem URL"));
        assert!(import("curl 'https://x")
            .unwrap_err()
            .to_string()
            .contains("aspas"));
        let cmd = "curl ^\"https://x/a^\" ^\n  -H ^\"accept: */*^\" ^\n  --compressed";
        assert!(import(cmd)
            .unwrap_err()
            .to_string()
            .contains("Copy as cURL (bash)"));
        assert!(import("curl -X BREW https://x")
            .unwrap_err()
            .to_string()
            .contains("BREW"));
    }

    /// Ida e volta: exportar sem valores e importar reproduz o request.
    #[test]
    fn export_then_import_round_trips() {
        let original = RequestFile {
            schema_version: 1,
            name: "POST orders".into(),
            method: HttpMethod::Post,
            url: "{{base}}/orders".into(),
            query: vec![kv("page", "1"), kv("filtro", "a b")],
            headers: vec![
                kv("x-api-key", "{{x-api-key}}"),
                kv("Content-Type", "application/json"),
            ],
            body: Body::Json {
                content: "{\n  \"nome\": \"D'Ávila\"\n}".into(),
            },
            settings: Settings {
                verify_tls: Some(false),
                ..Default::default()
            },
        };
        let exported = export(&original, None, false, CurlFormat::Bash).curl;
        assert_eq!(import(&exported).unwrap().request, original);
    }
}
