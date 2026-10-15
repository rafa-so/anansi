//! RNF-09: árvore com 500 requests carregada em menos de 1 s.
//!
//! Roda só em release: `npm run test:perf` (`cargo test --release -- --ignored`).

mod common;

use anansi_core::model::{Body, HttpMethod, RequestFile, Settings};
use anansi_core::Workspace;
use std::time::{Duration, Instant};

#[test]
#[ignore = "teste de desempenho; rodar com npm run test:perf"]
fn scans_500_requests_in_under_one_second() {
    let (_tmp, root) = common::empty_dir();
    let ws = Workspace::init(&root).unwrap();
    for c in 0..5 {
        let collection = ws
            .collection_create(&format!("Collection {c}"), None)
            .unwrap();
        for f in 0..10 {
            let folder = ws
                .folder_create(collection.path(), &format!("Pasta {f}"))
                .unwrap();
            for r in 0..10 {
                let request = RequestFile {
                    schema_version: 1,
                    name: format!("Request {r}"),
                    method: HttpMethod::Post,
                    url: "{{baseUrl}}/x".into(),
                    query: vec![],
                    headers: vec![],
                    body: Body::Json {
                        content: "{}".into(),
                    },
                    settings: Settings::default(),
                };
                ws.request_create(folder.path(), &request).unwrap();
            }
        }
    }
    let start = Instant::now();
    let tree = ws.tree();
    let elapsed = start.elapsed();
    assert_eq!(tree.iter().map(|n| n.request_count()).sum::<usize>(), 500);
    assert!(elapsed < Duration::from_secs(1), "levou {elapsed:?}");
}
