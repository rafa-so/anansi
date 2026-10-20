//! Execução HTTP contra servidores falsos (E4: RF-08, RF-09, RF-20, RNF-02, RNF-10, RNF-12).
//! Nenhum teste sai da máquina.

use anansi_core::env::{EnvironmentData, SecretValue};
use anansi_core::http::{BodyEncoding, Executor, HttpDefaults};
use anansi_core::model::{Body, FormField, HttpMethod, KeyValue, RequestFile, Settings};
use pretty_assertions::assert_eq;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{body_string, header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn kv(k: &str, v: &str) -> KeyValue {
    KeyValue {
        key: k.into(),
        value: v.into(),
        enabled: true,
    }
}

fn get(url: &str) -> RequestFile {
    RequestFile {
        schema_version: 1,
        name: "x".into(),
        method: HttpMethod::Get,
        url: url.into(),
        query: vec![],
        headers: vec![],
        body: Body::None,
        settings: Settings::default(),
    }
}

fn env(base: &str) -> EnvironmentData {
    EnvironmentData {
        variables: [
            ("base".to_string(), base.to_string()),
            ("produtoId".to_string(), "42".to_string()),
        ]
        .into(),
        secrets: [("token".to_string(), SecretValue::new("s3cr3t"))].into(),
    }
}

async fn send(
    exec: &Executor,
    request: &RequestFile,
    env: Option<&EnvironmentData>,
) -> anansi_core::AppResult<anansi_core::http::HttpResponse> {
    exec.send("exec-1", request, env, HttpDefaults::default())
        .await
}

/// V-09 (lógica): variáveis, segredos, query com encoding e JSON de resposta.
#[tokio::test]
async fn resolves_variables_and_sends_query_headers_and_json_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/orders"))
        .and(query_param("q", "a b"))
        .and(header("x-api-key", "s3cr3t"))
        .and(header("content-type", "application/json"))
        .and(body_string("{\"id\": 42}"))
        .respond_with(
            ResponseTemplate::new(201)
                .set_body_raw("{\"ok\":true}", "application/json")
                .append_header("Set-Cookie", "a=1")
                .append_header("Set-Cookie", "b=2"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let mut request = get("{{base}}/orders");
    request.method = HttpMethod::Post;
    request.query = vec![
        kv("q", "a b"),
        KeyValue {
            enabled: false,
            ..kv("off", "1")
        },
    ];
    request.headers = vec![kv("x-api-key", "{{token}}")];
    request.body = Body::Json {
        content: "{\"id\": {{produtoId}}}".into(),
    };

    let data = env(&server.uri());
    let response = send(&Executor::default(), &request, Some(&data))
        .await
        .unwrap();
    assert_eq!(response.status, 201);
    assert_eq!(response.status_text, "Created");
    assert_eq!(response.body, "{\"ok\":true}");
    assert_eq!(response.body_encoding, BodyEncoding::Utf8);
    assert_eq!(response.size_bytes, 11);
    assert!(response.tls_verified);
    let cookies: Vec<_> = response
        .headers
        .iter()
        .filter(|(k, _)| k == "set-cookie")
        .collect();
    assert_eq!(cookies.len(), 2, "headers repetidos são preservados");
    assert!(!response.url.contains("s3cr3t"));
}

#[tokio::test]
async fn explicit_content_type_wins_and_form_body_is_encoded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string("a=1+2&b=%C3%A7"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(header("content-type", "application/vnd.example+json"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let exec = Executor::default();
    let mut form = get(&server.uri());
    form.method = HttpMethod::Post;
    form.body = Body::FormUrlencoded {
        fields: vec![
            FormField::text("a", "1 2"),
            FormField::text("b", "ç"),
            FormField {
                enabled: false,
                ..FormField::text("c", "x")
            },
        ],
    };
    assert_eq!(send(&exec, &form, None).await.unwrap().status, 200);

    let mut put = get(&server.uri());
    put.method = HttpMethod::Put;
    put.headers = vec![kv("Content-Type", "application/vnd.example+json")];
    put.body = Body::Json {
        content: "{}".into(),
    };
    assert_eq!(send(&exec, &put, None).await.unwrap().status, 204);
}

#[tokio::test]
async fn empty_header_value_is_sent() {
    let server = MockServer::start().await;
    Mock::given(header_exists("x-api-key"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let mut request = get(&server.uri());
    request.headers = vec![kv("x-api-key", "")];
    assert_eq!(
        send(&Executor::default(), &request, None)
            .await
            .unwrap()
            .status,
        200
    );
}

#[tokio::test]
async fn error_statuses_are_normal_responses() {
    let server = MockServer::start().await;
    Mock::given(path("/x"))
        .respond_with(ResponseTemplate::new(503).set_body_string("fora"))
        .mount(&server)
        .await;
    let response = send(
        &Executor::default(),
        &get(&format!("{}/x", server.uri())),
        None,
    )
    .await
    .unwrap();
    assert_eq!(response.status, 503);
    assert_eq!(response.body, "fora");
}

#[tokio::test]
async fn follows_redirects_unless_disabled() {
    let server = MockServer::start().await;
    Mock::given(path("/old"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/new"))
        .mount(&server)
        .await;
    Mock::given(path("/new"))
        .respond_with(ResponseTemplate::new(200).set_body_string("novo"))
        .mount(&server)
        .await;
    Mock::given(path("/loop"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", "/loop"))
        .mount(&server)
        .await;

    let exec = Executor::default();
    let mut request = get(&format!("{}/old", server.uri()));
    assert_eq!(send(&exec, &request, None).await.unwrap().body, "novo");
    request.settings.follow_redirects = Some(false);
    assert_eq!(send(&exec, &request, None).await.unwrap().status, 302);

    let err = send(&exec, &get(&format!("{}/loop", server.uri())), None)
        .await
        .unwrap_err();
    assert_eq!(err.code(), "HTTP_ERROR");
    assert!(
        err.to_string().contains("Redirecionamentos demais"),
        "{err}"
    );
}

#[tokio::test]
async fn timeout_reports_original_url_without_secrets() {
    let server = MockServer::start().await;
    Mock::given(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let mut request = get("{{base}}/slow?t={{token}}");
    request.settings.timeout_ms = Some(200);
    let data = env(&server.uri());
    let err = send(&Executor::default(), &request, Some(&data))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "HTTP_ERROR");
    assert!(err.to_string().contains("Tempo esgotado"), "{err}");
    assert!(
        err.to_string().contains("{{base}}/slow"),
        "a mensagem usa a URL com placeholders: {err}"
    );
    assert!(!err.to_string().contains("s3cr3t"));
}

#[tokio::test]
async fn connection_refused_is_http_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let err = send(
        &Executor::default(),
        &get(&format!("http://127.0.0.1:{port}/x")),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(err.code(), "HTTP_ERROR");
    assert!(
        err.to_string().contains("Não foi possível conectar"),
        "{err}"
    );
}

#[tokio::test]
async fn invalid_urls() {
    let exec = Executor::default();
    assert_eq!(
        send(&exec, &get("{{naoExiste}}/x"), None)
            .await
            .unwrap_err()
            .code(),
        "INVALID_URL"
    );
    assert_eq!(
        send(&exec, &get("ftp://x/a"), None)
            .await
            .unwrap_err()
            .code(),
        "INVALID_URL"
    );
    assert_eq!(
        send(&exec, &get("http://"), None).await.unwrap_err().code(),
        "INVALID_URL"
    );
}

#[tokio::test]
async fn invalid_header_name_is_reported() {
    let server = MockServer::start().await;
    let mut request = get(&server.uri());
    request.headers = vec![kv("com espaço", "1")];
    let err = send(&Executor::default(), &request, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Header inválido"), "{err}");
}

/// V-23: body acima do limite de exibição é truncado, mas guardado inteiro para salvar.
#[tokio::test]
async fn truncates_large_body_and_saves_full_content() {
    let server = MockServer::start().await;
    let big = "ç".repeat(600); // 1200 bytes
    Mock::given(path("/big"))
        .respond_with(ResponseTemplate::new(200).set_body_string(big.clone()))
        .mount(&server)
        .await;
    let exec = Executor::with_limits(1001, 10_000);
    let response = exec
        .send(
            "e",
            &get(&format!("{}/big", server.uri())),
            None,
            HttpDefaults::default(),
        )
        .await
        .unwrap();
    assert!(response.body_truncated);
    assert_eq!(response.body.len(), 1000, "corta na borda de caractere");
    assert_eq!(response.size_bytes, 1200);

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("resposta.txt");
    exec.save_body("e", &dest).unwrap();
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), big);
    exec.release("e");
    assert_eq!(exec.save_body("e", &dest).unwrap_err().code(), "NOT_FOUND");
}

#[tokio::test]
async fn too_large_response_is_cut() {
    let server = MockServer::start().await;
    Mock::given(path("/huge"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 5000]))
        .mount(&server)
        .await;
    let exec = Executor::with_limits(100, 4096);
    let err = exec
        .send(
            "e",
            &get(&format!("{}/huge", server.uri())),
            None,
            HttpDefaults::default(),
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "RESPONSE_TOO_LARGE");
}

#[tokio::test]
async fn binary_body_is_flagged() {
    let server = MockServer::start().await;
    Mock::given(path("/img"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(vec![0xff, 0xd8, 0xff, 0x00, 0x10], "image/jpeg"),
        )
        .mount(&server)
        .await;
    let response = send(
        &Executor::default(),
        &get(&format!("{}/img", server.uri())),
        None,
    )
    .await
    .unwrap();
    assert_eq!(response.body_encoding, BodyEncoding::Binary);
    assert_eq!(response.body, "");
    assert_eq!(response.size_bytes, 5);
}

#[tokio::test]
async fn gzip_is_decompressed_and_size_is_uncompressed() {
    let server = MockServer::start().await;
    let text = "{\"data\":\"".to_string() + &"a".repeat(1000) + "\"}";
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(text.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();
    Mock::given(path("/gz"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(compressed, "application/json")
                .insert_header("Content-Encoding", "gzip"),
        )
        .mount(&server)
        .await;
    let response = send(
        &Executor::default(),
        &get(&format!("{}/gz", server.uri())),
        None,
    )
    .await
    .unwrap();
    assert_eq!(response.body, text);
    assert_eq!(response.size_bytes, text.len());
}

/// V-22: cancelar uma requisição lenta.
#[tokio::test]
async fn cancels_running_request() {
    let server = MockServer::start().await;
    Mock::given(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(10)))
        .mount(&server)
        .await;
    let exec = Arc::new(Executor::default());
    let request = get(&format!("{}/slow", server.uri()));
    let running = {
        let exec = exec.clone();
        tokio::spawn(async move {
            exec.send("lento", &request, None, HttpDefaults::default())
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(exec.is_running("lento"));
    exec.cancel("lento");
    let err = running.await.unwrap().unwrap_err();
    assert_eq!(err.code(), "CANCELLED");
    assert!(!exec.is_running("lento"));
}

#[tokio::test]
async fn request_headers_are_received_by_server() {
    let server = MockServer::start().await;
    Mock::given(path("/echo"))
        .respond_with(|req: &Request| {
            let values: Vec<_> = req
                .headers
                .get_all("x-repetido")
                .iter()
                .map(|v| v.to_str().unwrap().to_string())
                .collect();
            ResponseTemplate::new(200).set_body_string(values.join(","))
        })
        .mount(&server)
        .await;
    let mut request = get(&format!("{}/echo", server.uri()));
    request.headers = vec![kv("X-Repetido", "1"), kv("X-Repetido", "2")];
    assert_eq!(
        send(&Executor::default(), &request, None)
            .await
            .unwrap()
            .body,
        "1,2"
    );
}

// ----- TLS (RNF-12) -----

/// Servidor HTTPS mínimo com certificado autoassinado; responde "ok" a qualquer request.
async fn start_self_signed_server() -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::rustls;

    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert_der = rustls::pki_types::CertificateDer::from(cert.cert.der().to_vec());
    let key_der =
        rustls::pki_types::PrivateKeyDer::try_from(cert.key_pair.serialize_der()).unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![cert_der], key_der)
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(stream).await else {
                    return;
                };
                let mut buf = [0u8; 4096];
                let _ = tls.read(&mut buf).await;
                let _ = tls
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                    )
                    .await;
                let _ = tls.shutdown().await;
            });
        }
    });
    format!("https://localhost:{port}/")
}

/// V-24: certificado autoassinado aceito com verifyTls desligado, recusado com ligado.
#[tokio::test]
async fn self_signed_certificate_depends_on_verify_tls() {
    let url = start_self_signed_server().await;
    let exec = Executor::default();
    let mut request = get(&url);

    let err = send(&exec, &request, None).await.unwrap_err();
    assert_eq!(
        err.code(),
        "HTTP_ERROR",
        "com verifyTls ligado o certificado é recusado"
    );

    request.settings.verify_tls = Some(false);
    let response = send(&exec, &request, None).await.unwrap();
    assert_eq!(response.body, "ok");
    assert!(!response.tls_verified);
}

#[tokio::test]
async fn global_default_applies_when_request_does_not_define() {
    let url = start_self_signed_server().await;
    let defaults = HttpDefaults {
        verify_tls: false,
        ..HttpDefaults::default()
    };
    let response = Executor::default()
        .send("e", &get(&url), None, defaults)
        .await
        .unwrap();
    assert!(!response.tls_verified);
}

/// Formulário com arquivo sai como multipart/form-data, com o conteúdo do arquivo.
#[tokio::test]
async fn form_with_file_is_sent_as_multipart() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|req: &Request| {
            let ct = req
                .headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            ResponseTemplate::new(200)
                .set_body_string(format!("{ct}\n{}", String::from_utf8_lossy(&req.body)))
        })
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("nota.txt");
    std::fs::write(&file, "conteúdo do arquivo").unwrap();

    let mut form = get(&server.uri());
    form.method = HttpMethod::Post;
    form.body = Body::FormUrlencoded {
        fields: vec![
            FormField::text("nome", "Ana"),
            FormField::file("anexo", file.to_str().unwrap()),
        ],
    };
    let response = send(&Executor::default(), &form, None).await.unwrap();
    assert_eq!(response.status, 200);
    let (ct, body) = response.body.split_once('\n').unwrap();
    let boundary = ct
        .strip_prefix("multipart/form-data; boundary=")
        .expect("content-type multipart com boundary");
    assert!(body.contains("Content-Disposition: form-data; name=\"nome\"\r\n\r\nAna\r\n"));
    assert!(body.contains(
        "Content-Disposition: form-data; name=\"anexo\"; filename=\"nota.txt\"\r\nContent-Type: text/plain\r\n\r\nconteúdo do arquivo\r\n"
    ));
    assert!(body.ends_with(&format!("--{boundary}--\r\n")));
}

#[tokio::test]
async fn missing_form_file_is_reported_before_sending() {
    let mut form = get("http://127.0.0.1:9/x");
    form.method = HttpMethod::Post;
    form.body = Body::FormUrlencoded {
        fields: vec![FormField::file("anexo", "/nao/existe/arquivo.pdf")],
    };
    let err = send(&Executor::default(), &form, None).await.unwrap_err();
    assert_eq!(err.code(), "INVALID_FILE");
    assert!(err.to_string().contains("anexo"));
}
