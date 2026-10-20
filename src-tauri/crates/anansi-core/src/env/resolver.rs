//! Resolução de `{{nome}}` (Seção 6 da spec).
//!
//! - Sintaxe: `{{ nome }}`, com nome `[A-Za-z0-9_.-]+` e espaços opcionais.
//! - Uma única passada: um valor que contenha `{{outra}}` não é resolvido de novo.
//! - Fontes: variáveis e segredos do ambiente ativo; em nome duplicado, vale o segredo.

use super::{is_valid_var_name, EnvironmentData};
use crate::model::{Body, FormField, KeyValue, RequestFile};
use std::collections::BTreeSet;

/// Placeholder encontrado no texto: posição em bytes e nome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placeholder {
    pub start: usize,
    pub end: usize,
    pub name: String,
}

/// Encontra todos os `{{nome}}` válidos do texto.
pub fn placeholders(text: &str) -> Vec<Placeholder> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find("{{").map(|i| i + from) {
        let Some(close) = text[open + 2..].find("}}").map(|i| i + open + 2) else {
            break;
        };
        let inner = &text[open + 2..close];
        let name = inner.trim();
        if is_valid_var_name(name) && !inner.contains('{') {
            found.push(Placeholder {
                start: open,
                end: close + 2,
                name: name.to_string(),
            });
            from = close + 2;
        } else {
            from = open + 1;
        }
    }
    found
}

/// Resolve placeholders com os valores de um ambiente.
pub struct Resolver<'a> {
    env: Option<&'a EnvironmentData>,
}

/// Resultado acumulado de várias resoluções.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub unresolved: BTreeSet<String>,
    pub used_secret: bool,
}

impl<'a> Resolver<'a> {
    /// Sem ambiente ativo, nada é resolvido.
    pub fn new(env: Option<&'a EnvironmentData>) -> Self {
        Resolver { env }
    }

    fn lookup(&self, name: &str) -> Option<(&'a str, bool)> {
        let env = self.env?;
        if let Some(secret) = env.secrets.get(name) {
            return Some((secret.expose(), true));
        }
        env.variables.get(name).map(|v| (v.as_str(), false))
    }

    /// Substitui os placeholders conhecidos; os desconhecidos ficam como estão.
    pub fn resolve(&self, text: &str, report: &mut Report) -> String {
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for p in placeholders(text) {
            out.push_str(&text[last..p.start]);
            match self.lookup(&p.name) {
                Some((value, secret)) => {
                    out.push_str(value);
                    report.used_secret |= secret;
                }
                None => {
                    out.push_str(&text[p.start..p.end]);
                    report.unresolved.insert(p.name);
                }
            }
            last = p.end;
        }
        out.push_str(&text[last..]);
        out
    }

    fn resolve_list(&self, items: &[KeyValue], report: &mut Report) -> Vec<KeyValue> {
        items
            .iter()
            .filter(|kv| kv.enabled)
            .map(|kv| KeyValue {
                key: self.resolve(&kv.key, report),
                value: self.resolve(&kv.value, report),
                enabled: true,
            })
            .collect()
    }

    /// Como `resolve_list`, mantendo o tipo do campo (no arquivo, resolve o caminho).
    fn resolve_form(&self, fields: &[FormField], report: &mut Report) -> Vec<FormField> {
        fields
            .iter()
            .filter(|f| f.enabled)
            .map(|f| FormField {
                key: self.resolve(&f.key, report),
                value: self.resolve(&f.value, report),
                enabled: true,
                kind: f.kind,
            })
            .collect()
    }

    /// Resolve URL, query, headers e body (Seção 6.2). Itens desabilitados são descartados.
    pub fn resolve_request(&self, request: &RequestFile) -> (RequestFile, Report) {
        let mut report = Report::default();
        let mut resolved = request.clone();
        resolved.url = self.resolve(&request.url, &mut report);
        resolved.query = self.resolve_list(&request.query, &mut report);
        resolved.headers = self.resolve_list(&request.headers, &mut report);
        resolved.body = match &request.body {
            Body::Json { content } => Body::Json {
                content: self.resolve(content, &mut report),
            },
            Body::Text { content } => Body::Text {
                content: self.resolve(content, &mut report),
            },
            Body::FormUrlencoded { fields } => Body::FormUrlencoded {
                fields: self.resolve_form(fields, &mut report),
            },
            Body::None => Body::None,
        };
        (resolved, report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::SecretValue;
    use crate::model::{HttpMethod, Settings};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    fn env() -> EnvironmentData {
        EnvironmentData {
            variables: [
                ("baseUrl".to_string(), "http://localhost:8080".to_string()),
                ("loop".to_string(), "{{baseUrl}}".to_string()),
                ("token".to_string(), "da-variavel".to_string()),
            ]
            .into(),
            secrets: [("token".to_string(), SecretValue::new("s3cr3t"))].into(),
        }
    }

    #[rstest]
    #[case("{{baseUrl}}/x", "http://localhost:8080/x")]
    #[case("{{ baseUrl }}/x", "http://localhost:8080/x")]
    #[case("{{loop}}", "{{baseUrl}}")]
    #[case("{{naoExiste}}", "{{naoExiste}}")]
    #[case("{{com espaço}}", "{{com espaço}}")]
    #[case("{baseUrl}", "{baseUrl}")]
    #[case("{{{baseUrl}}}", "{http://localhost:8080}")]
    #[case(
        "a{{baseUrl}}b{{baseUrl}}",
        "ahttp://localhost:8080bhttp://localhost:8080"
    )]
    #[case("{{", "{{")]
    #[case("ação {{baseUrl}}", "ação http://localhost:8080")]
    fn resolves_text(#[case] text: &str, #[case] expected: &str) {
        let data = env();
        let mut report = Report::default();
        assert_eq!(
            Resolver::new(Some(&data)).resolve(text, &mut report),
            expected
        );
    }

    #[test]
    fn secret_wins_and_is_reported() {
        let data = env();
        let mut report = Report::default();
        assert_eq!(
            Resolver::new(Some(&data)).resolve("Bearer {{token}}", &mut report),
            "Bearer s3cr3t"
        );
        assert!(report.used_secret);
    }

    #[test]
    fn without_environment_nothing_is_resolved() {
        let mut report = Report::default();
        assert_eq!(
            Resolver::new(None).resolve("{{baseUrl}}", &mut report),
            "{{baseUrl}}"
        );
        assert_eq!(
            report.unresolved.into_iter().collect::<Vec<_>>(),
            vec!["baseUrl"]
        );
    }

    #[test]
    fn placeholders_report_positions() {
        assert_eq!(
            placeholders("x {{ a }} {{b}}"),
            vec![
                Placeholder {
                    start: 2,
                    end: 9,
                    name: "a".into()
                },
                Placeholder {
                    start: 10,
                    end: 15,
                    name: "b".into()
                }
            ]
        );
    }

    #[test]
    fn resolves_whole_request_and_drops_disabled_items() {
        let data = env();
        let request = RequestFile {
            schema_version: 1,
            name: "X".into(),
            method: HttpMethod::Post,
            url: "{{baseUrl}}/orders".into(),
            query: vec![
                KeyValue {
                    key: "id".into(),
                    value: "{{id}}".into(),
                    enabled: true,
                },
                KeyValue {
                    key: "off".into(),
                    value: "{{desligado}}".into(),
                    enabled: false,
                },
            ],
            headers: vec![KeyValue {
                key: "x-api-key".into(),
                value: "{{token}}".into(),
                enabled: true,
            }],
            body: Body::Json {
                content: "{\"url\": \"{{baseUrl}}\"}".into(),
            },
            settings: Settings::default(),
        };
        let (resolved, report) = Resolver::new(Some(&data)).resolve_request(&request);
        assert_eq!(resolved.url, "http://localhost:8080/orders");
        assert_eq!(resolved.query.len(), 1, "desabilitados não são enviados");
        assert_eq!(resolved.headers[0].value, "s3cr3t");
        assert_eq!(
            resolved.body,
            Body::Json {
                content: "{\"url\": \"http://localhost:8080\"}".into()
            }
        );
        assert_eq!(
            report.unresolved.into_iter().collect::<Vec<_>>(),
            vec!["id"],
            "desligados não contam"
        );
        assert!(report.used_secret);
    }
}
