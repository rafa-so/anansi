//! Execução de requests HTTP (Seção 8 da spec).
//!
//! As requisições saem pelo `reqwest` (camada nativa, sem CORS, RNF-02). Cada
//! execução tem um `execId` gerado pelo frontend, usado para cancelar e para
//! salvar o body completo depois.

use crate::env::resolver::Resolver;
use crate::env::EnvironmentData;
use crate::error::{AppError, AppResult};
use crate::model::{Body, FormField, KeyValue, RequestFile};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Limite do body exibido na interface (Seção 8.2).
pub const DISPLAY_LIMIT: usize = 2 * 1024 * 1024;
/// Limite de download: acima disso a leitura é interrompida.
pub const DOWNLOAD_LIMIT: usize = 50 * 1024 * 1024;
pub const MAX_REDIRECTS: usize = 10;

/// Padrões das preferências globais, usados quando o request não define (Seção 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpDefaults {
    pub verify_tls: bool,
    pub timeout_ms: u64,
    pub follow_redirects: bool,
}

impl Default for HttpDefaults {
    fn default() -> Self {
        HttpDefaults {
            verify_tls: true,
            timeout_ms: 30_000,
            follow_redirects: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyEncoding {
    Utf8,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    /// Lista, para preservar repetições (ex.: `Set-Cookie`).
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub body_encoding: BodyEncoding,
    pub body_truncated: bool,
    /// Tamanho real do body recebido (descomprimido).
    pub size_bytes: usize,
    pub elapsed_ms: u64,
    pub tls_verified: bool,
    /// Placeholders que não foram resolvidos e seguiram como texto literal.
    pub unresolved: Vec<String>,
    /// URL final efetivamente chamada, com os segredos mascarados.
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ClientKey {
    verify_tls: bool,
    follow_redirects: bool,
}

#[derive(Clone, Copy)]
struct Limits {
    display: usize,
    download: usize,
}

/// Executor com clientes reaproveitados, execuções em andamento e bodies guardados.
pub struct Executor {
    clients: Mutex<HashMap<ClientKey, reqwest::Client>>,
    running: Mutex<HashMap<String, tokio::task::AbortHandle>>,
    bodies: Mutex<HashMap<String, Arc<Vec<u8>>>>,
    limits: Limits,
}

impl Default for Executor {
    fn default() -> Self {
        Executor::with_limits(DISPLAY_LIMIT, DOWNLOAD_LIMIT)
    }
}

/// Request já resolvido e pronto para enviar.
struct Prepared {
    method: reqwest::Method,
    url: url::Url,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    key: ClientKey,
    timeout: Duration,
    display_url: String,
    masked_url: String,
    unresolved: Vec<String>,
}

impl Executor {
    /// Limites menores servem para testar truncamento e corte sem respostas enormes.
    pub fn with_limits(display: usize, download: usize) -> Self {
        Executor {
            clients: Mutex::new(HashMap::new()),
            running: Mutex::new(HashMap::new()),
            bodies: Mutex::new(HashMap::new()),
            limits: Limits { display, download },
        }
    }

    fn client(&self, key: ClientKey) -> AppResult<reqwest::Client> {
        let mut clients = self.clients.lock().unwrap();
        if let Some(c) = clients.get(&key) {
            return Ok(c.clone());
        }
        let redirect = if key.follow_redirects {
            reqwest::redirect::Policy::limited(MAX_REDIRECTS)
        } else {
            reqwest::redirect::Policy::none()
        };
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .danger_accept_invalid_certs(!key.verify_tls)
            .redirect(redirect)
            .user_agent(concat!("anansi/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| {
                AppError::HttpError(format!(
                    "Falha ao preparar o cliente HTTP: {}",
                    error_chain(&e)
                ))
            })?;
        clients.insert(key, client.clone());
        Ok(client)
    }

    /// Envia o rascunho da aba (RF-08). `env` é o ambiente ativo, se houver.
    pub async fn send(
        &self,
        exec_id: &str,
        request: &RequestFile,
        env: Option<&EnvironmentData>,
        defaults: HttpDefaults,
    ) -> AppResult<HttpResponse> {
        self.release(exec_id);
        let prepared = prepare(request, env, defaults)?;
        let client = self.client(prepared.key)?;
        let limits = self.limits;
        let verify_tls = prepared.key.verify_tls;
        let display_url = prepared.display_url.clone();
        let masked_url = prepared.masked_url.clone();
        let unresolved = prepared.unresolved.clone();
        let timeout = prepared.timeout;

        let task = tokio::spawn(async move { execute(client, prepared, limits).await });
        self.running
            .lock()
            .unwrap()
            .insert(exec_id.to_string(), task.abort_handle());
        let outcome = task.await;
        self.running.lock().unwrap().remove(exec_id);

        let raw = match outcome {
            Err(join) if join.is_cancelled() => {
                return Err(AppError::Cancelled("Requisição cancelada.".into()))
            }
            Err(join) => {
                return Err(AppError::HttpError(format!(
                    "Falha inesperada na execução: {join}"
                )))
            }
            Ok(result) => result.map_err(|e| e.into_app_error(&display_url, timeout, limits))?,
        };

        let size_bytes = raw.body.len();
        let (body, body_encoding, body_truncated) = match std::str::from_utf8(&raw.body) {
            Ok(text) if text.len() > limits.display => (
                truncate_utf8(text, limits.display).to_string(),
                BodyEncoding::Utf8,
                true,
            ),
            Ok(text) => (text.to_string(), BodyEncoding::Utf8, false),
            Err(e) if e.error_len().is_none() && raw.body.len() > limits.display => {
                // Corte no meio de um caractere multibyte: o conteúdo é texto.
                let text = std::str::from_utf8(&raw.body[..e.valid_up_to()]).unwrap_or_default();
                (
                    truncate_utf8(text, limits.display).to_string(),
                    BodyEncoding::Utf8,
                    true,
                )
            }
            Err(_) => (String::new(), BodyEncoding::Binary, false),
        };
        self.bodies
            .lock()
            .unwrap()
            .insert(exec_id.to_string(), Arc::new(raw.body));

        Ok(HttpResponse {
            status: raw.status.as_u16(),
            status_text: raw.status.canonical_reason().unwrap_or("").to_string(),
            headers: raw.headers,
            body,
            body_encoding,
            body_truncated,
            size_bytes,
            elapsed_ms: raw.elapsed.as_millis() as u64,
            tls_verified: verify_tls,
            unresolved,
            url: masked_url,
        })
    }

    /// Cancela a execução em andamento (RF-20). Sem efeito se já terminou.
    pub fn cancel(&self, exec_id: &str) {
        if let Some(handle) = self.running.lock().unwrap().remove(exec_id) {
            handle.abort();
        }
    }

    /// Grava o body completo da última resposta da execução (RNF-10).
    pub fn save_body(&self, exec_id: &str, dest: &Path) -> AppResult<()> {
        let body = self
            .bodies
            .lock()
            .unwrap()
            .get(exec_id)
            .cloned()
            .ok_or_else(|| {
                AppError::NotFound(
                    "A resposta não está mais disponível. Envie o request de novo.".into(),
                )
            })?;
        std::fs::write(dest, body.as_slice()).map_err(|e| AppError::io(&e, dest))
    }

    /// Libera o body guardado (aba fechada ou nova execução).
    pub fn release(&self, exec_id: &str) {
        self.bodies.lock().unwrap().remove(exec_id);
    }

    pub fn is_running(&self, exec_id: &str) -> bool {
        self.running.lock().unwrap().contains_key(exec_id)
    }
}

/// Tamanho legível: `512 B`, `1,5 KB`, `50 MB`.
pub fn human_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    let text = if b < KB {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{:.1} MB", b / (KB * KB))
    };
    text.replace(".0 ", " ").replace('.', ",")
}

fn truncate_utf8(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Mostra o erro e as causas, sem a URL (que pode conter segredos já resolvidos).
fn error_chain(err: &(dyn std::error::Error + 'static)) -> String {
    let mut parts = vec![err.to_string()];
    let mut source = err.source();
    while let Some(s) = source {
        let text = s.to_string();
        if !parts.iter().any(|p| p.contains(&text)) {
            parts.push(text);
        }
        source = s.source();
    }
    parts.join(": ")
}

/// Sem esquema, assume `http://` (ex.: `localhost:8080/x`).
fn with_scheme(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}

/// Monta a URL final: URL resolvida + query habilitada com percent-encoding.
pub fn build_url(url: &str, query: &[KeyValue]) -> Result<url::Url, url::ParseError> {
    let mut parsed = url::Url::parse(&with_scheme(url))?;
    let enabled: Vec<&KeyValue> = query
        .iter()
        .filter(|q| q.enabled && !(q.key.is_empty() && q.value.is_empty()))
        .collect();
    if !enabled.is_empty() {
        let mut pairs = parsed.query_pairs_mut();
        for q in enabled {
            pairs.append_pair(&q.key, &q.value);
        }
    }
    Ok(parsed)
}

fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name))
}

/// `Content-Type` automático por tipo de body (RF-09), quando o usuário não definiu.
///
/// Formulário com arquivo vira `multipart/form-data`; no envio, o `boundary` é acrescentado.
pub fn automatic_content_type(body: &Body) -> Option<&'static str> {
    match body {
        Body::Json { .. } => Some("application/json"),
        Body::Text { .. } => Some("text/plain; charset=utf-8"),
        Body::FormUrlencoded { .. } if body.has_files() => Some("multipart/form-data"),
        Body::FormUrlencoded { .. } => Some("application/x-www-form-urlencoded"),
        Body::None => None,
    }
}

/// Codifica os campos habilitados de um form-urlencoded.
pub fn encode_form(fields: &[FormField]) -> String {
    let mut ser = url::form_urlencoded::Serializer::new(String::new());
    for f in fields.iter().filter(|f| f.enabled) {
        ser.append_pair(&f.key, &f.value);
    }
    ser.finish()
}

/// Separador único por envio: horário em nanossegundos + contador.
fn new_boundary() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("----anansi{nanos:x}{n:x}")
}

/// Nome dentro de `Content-Disposition`: aspas e quebras de linha escapadas (como os navegadores).
fn disposition_name(text: &str) -> String {
    text.replace('"', "%22")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// `Content-Type` da parte de arquivo, pela extensão. Desconhecida → `application/octet-stream`.
pub fn mime_for_path(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "csv" => "text/csv",
        "txt" | "log" => "text/plain",
        "html" | "htm" => "text/html",
        "md" => "text/markdown",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

/// Monta um `multipart/form-data` com os campos habilitados, lendo os arquivos do disco.
/// Devolve o `boundary` e o corpo.
pub fn encode_multipart(fields: &[FormField]) -> AppResult<(String, Vec<u8>)> {
    let boundary = new_boundary();
    let mut out: Vec<u8> = Vec::new();
    for f in fields.iter().filter(|f| f.enabled) {
        out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        let name = disposition_name(&f.key);
        if f.is_file() {
            let path = f.value.trim();
            if path.is_empty() {
                return Err(AppError::InvalidFile(format!(
                    "O campo \"{}\" é do tipo arquivo, mas nenhum arquivo foi escolhido.",
                    f.key
                )));
            }
            let content = std::fs::read(path).map_err(|e| {
                AppError::InvalidFile(format!(
                    "Não foi possível ler o arquivo do campo \"{}\" ({path}): {e}",
                    f.key
                ))
            })?;
            let file_name = Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string());
            out.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{name}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                    disposition_name(&file_name),
                    mime_for_path(path)
                )
                .as_bytes(),
            );
            out.extend_from_slice(&content);
        } else {
            out.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
            );
            out.extend_from_slice(f.value.as_bytes());
        }
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    Ok((boundary, out))
}

fn prepare(
    request: &RequestFile,
    env: Option<&EnvironmentData>,
    defaults: HttpDefaults,
) -> AppResult<Prepared> {
    let resolver = Resolver::new(env);
    let (resolved, report) = resolver.resolve_request(request);
    let display_url = request.url.clone();

    let url = build_url(&resolved.url, &resolved.query)
        .map_err(|e| AppError::InvalidUrl(format!("URL inválida: {display_url} ({e})")))?;
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    if host.is_empty() || host.contains('{') || host.contains("%7b") {
        return Err(AppError::InvalidUrl(format!(
            "URL inválida: {display_url} (o endereço ficou sem servidor; confira o ambiente ativo e as variáveis)"
        )));
    }
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::InvalidUrl(format!(
            "URL inválida: {display_url} (use http:// ou https://)"
        )));
    }
    // URL para exibir: resolvida, mas com os segredos de volta como placeholders.
    let masked_url = {
        let mut masked = request.clone();
        masked.url = request.url.clone();
        let no_secrets = env.map(|e| EnvironmentData {
            variables: e.variables.clone(),
            secrets: Default::default(),
        });
        let (m, _) = Resolver::new(no_secrets.as_ref()).resolve_request(&masked);
        build_url(&m.url, &m.query)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| m.url)
    };

    let mut headers: Vec<(String, String)> = resolved
        .headers
        .iter()
        .filter(|h| !h.key.trim().is_empty())
        .map(|h| (h.key.trim().to_string(), h.value.clone()))
        .collect();
    let body = match &resolved.body {
        Body::None => None,
        Body::Json { content } | Body::Text { content } => Some(content.clone().into_bytes()),
        Body::FormUrlencoded { fields } if resolved.body.has_files() => {
            // O boundary é gerado aqui, então o Content-Type do usuário (sem boundary) é trocado.
            let (boundary, bytes) = encode_multipart(fields)?;
            headers.retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
            headers.push((
                "Content-Type".into(),
                format!("multipart/form-data; boundary={boundary}"),
            ));
            Some(bytes)
        }
        Body::FormUrlencoded { fields } => Some(encode_form(fields).into_bytes()),
    };
    if let Some(ct) = automatic_content_type(&resolved.body) {
        if !has_header(&headers, "content-type") {
            headers.push(("Content-Type".into(), ct.into()));
        }
    }

    let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
        .expect("métodos do modelo são válidos");
    let settings = &request.settings;
    let key = ClientKey {
        verify_tls: settings.verify_tls.unwrap_or(defaults.verify_tls),
        follow_redirects: settings
            .follow_redirects
            .unwrap_or(defaults.follow_redirects),
    };
    let timeout = Duration::from_millis(settings.timeout_ms.unwrap_or(defaults.timeout_ms).max(1));
    Ok(Prepared {
        method,
        url,
        headers,
        body,
        key,
        timeout,
        display_url,
        masked_url,
        unresolved: report.unresolved.into_iter().collect(),
    })
}

struct RawResponse {
    status: reqwest::StatusCode,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    elapsed: Duration,
}

enum ExecError {
    Request(reqwest::Error),
    Header(String),
    TooLarge,
}

impl ExecError {
    fn into_app_error(self, display_url: &str, timeout: Duration, limits: Limits) -> AppError {
        match self {
            ExecError::TooLarge => AppError::ResponseTooLarge(format!(
                "A resposta passou de {} e a leitura foi interrompida.",
                human_size(limits.download)
            )),
            ExecError::Header(name) => AppError::HttpError(format!("Header inválido: \"{name}\".")),
            ExecError::Request(e) => {
                let e = e.without_url();
                let detail = error_chain(&e);
                let msg = if e.is_timeout() {
                    format!(
                        "Tempo esgotado ({} s) ao chamar {display_url}.",
                        timeout.as_secs_f64()
                    )
                } else if e.is_connect() {
                    format!("Não foi possível conectar a {display_url}: {detail}")
                } else if e.is_redirect() {
                    format!("Redirecionamentos demais ao chamar {display_url} (máximo {MAX_REDIRECTS}).")
                } else {
                    format!("Falha ao chamar {display_url}: {detail}")
                };
                AppError::HttpError(msg)
            }
        }
    }
}

async fn execute(
    client: reqwest::Client,
    p: Prepared,
    limits: Limits,
) -> Result<RawResponse, ExecError> {
    let mut header_map = reqwest::header::HeaderMap::new();
    for (k, v) in &p.headers {
        let name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
            .map_err(|_| ExecError::Header(k.clone()))?;
        let value =
            reqwest::header::HeaderValue::from_str(v).map_err(|_| ExecError::Header(k.clone()))?;
        header_map.append(name, value);
    }
    let mut builder = client
        .request(p.method, p.url)
        .headers(header_map)
        .timeout(p.timeout);
    if let Some(body) = p.body {
        builder = builder.body(body);
    }
    let start = Instant::now();
    let mut response = builder.send().await.map_err(ExecError::Request)?;
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                String::from_utf8_lossy(v.as_bytes()).into_owned(),
            )
        })
        .collect();
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(ExecError::Request)? {
        if body.len() + chunk.len() > limits.download {
            return Err(ExecError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(RawResponse {
        status,
        headers,
        body,
        elapsed: start.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HttpMethod, Settings};

    fn kv(k: &str, v: &str) -> KeyValue {
        KeyValue {
            key: k.into(),
            value: v.into(),
            enabled: true,
        }
    }

    #[test]
    fn builds_url_with_encoded_query() {
        let url = build_url(
            "http://x/a",
            &[
                kv("q", "a b&c"),
                kv("vazio", ""),
                KeyValue {
                    enabled: false,
                    ..kv("off", "1")
                },
            ],
        )
        .unwrap();
        assert_eq!(url.as_str(), "http://x/a?q=a+b%26c&vazio=");
    }

    #[test]
    fn missing_scheme_defaults_to_http() {
        assert_eq!(
            build_url("localhost:8080/orders", &[]).unwrap().as_str(),
            "http://localhost:8080/orders"
        );
    }

    #[test]
    fn unresolved_placeholder_in_host_is_invalid_url_with_original_text() {
        let request = RequestFile {
            schema_version: 1,
            name: "x".into(),
            method: HttpMethod::Get,
            url: "{{base_url}}/x".into(),
            query: vec![],
            headers: vec![],
            body: Body::None,
            settings: Settings::default(),
        };
        let err = prepare(&request, None, HttpDefaults::default())
            .err()
            .unwrap();
        assert_eq!(err.code(), "INVALID_URL");
        assert!(err.to_string().contains("{{base_url}}/x"));
    }

    #[test]
    fn human_sizes() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1536), "1,5 KB");
        assert_eq!(human_size(50 * 1024 * 1024), "50 MB");
    }

    #[test]
    fn truncates_on_char_boundary() {
        assert_eq!(truncate_utf8("aç", 2), "a");
        assert_eq!(truncate_utf8("abc", 5), "abc");
    }

    #[test]
    fn form_encoding_skips_disabled_fields() {
        let fields = [
            FormField::text("a", "1 2"),
            FormField {
                enabled: false,
                ..FormField::text("b", "x")
            },
            FormField::text("c", "ç"),
        ];
        assert_eq!(encode_form(&fields), "a=1+2&c=%C3%A7");
    }

    #[test]
    fn multipart_has_text_and_file_parts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foto \"1\".png");
        std::fs::write(&path, [0u8, 159, 146, 150]).unwrap();
        let fields = [
            FormField::text("nome", "Ana"),
            FormField::file("foto", path.to_str().unwrap()),
            FormField {
                enabled: false,
                ..FormField::text("off", "x")
            },
        ];
        let (boundary, body) = encode_multipart(&fields).unwrap();
        let mut expected = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"nome\"\r\n\r\nAna\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"foto\"; filename=\"foto %221%22.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .into_bytes();
        expected.extend_from_slice(&[0u8, 159, 146, 150]);
        expected.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        assert_eq!(body, expected);
        assert_ne!(
            encode_multipart(&fields).unwrap().0,
            boundary,
            "boundary novo a cada envio"
        );
    }

    #[test]
    fn multipart_errors_for_missing_or_empty_file() {
        let err = encode_multipart(&[FormField::file("doc", "  ")]).unwrap_err();
        assert_eq!(err.code(), "INVALID_FILE");
        assert!(err.to_string().contains("nenhum arquivo foi escolhido"));
        let err = encode_multipart(&[FormField::file("doc", "/nao/existe.pdf")]).unwrap_err();
        assert!(err.to_string().contains("/nao/existe.pdf"));
    }

    #[test]
    fn multipart_replaces_user_content_type_with_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "oi").unwrap();
        let request = RequestFile {
            schema_version: 1,
            name: "x".into(),
            method: HttpMethod::Post,
            url: "http://x/upload".into(),
            query: vec![],
            headers: vec![kv("content-type", "multipart/form-data")],
            body: Body::FormUrlencoded {
                fields: vec![FormField::file("a", path.to_str().unwrap())],
            },
            settings: Settings::default(),
        };
        let prepared = prepare(&request, None, HttpDefaults::default())
            .ok()
            .unwrap();
        let cts: Vec<_> = prepared
            .headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .collect();
        assert_eq!(cts.len(), 1);
        assert!(cts[0]
            .1
            .starts_with("multipart/form-data; boundary=----anansi"));
    }

    #[test]
    fn mime_by_extension() {
        assert_eq!(mime_for_path("C:/a/B.PNG"), "image/png");
        assert_eq!(mime_for_path("/x/arquivo"), "application/octet-stream");
    }
}
