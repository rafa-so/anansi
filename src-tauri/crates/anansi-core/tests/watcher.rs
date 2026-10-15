//! File watcher (E7: RF-17). Prazos folgados para não ficar instável.

mod common;

use anansi_core::events::{Event, MemorySink};
use anansi_core::model::{Body, HttpMethod, RequestFile, Settings};
use anansi_core::watcher::WorkspaceWatcher;
use anansi_core::Workspace;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn changed_paths(sink: &MemorySink, timeout: Duration) -> Vec<String> {
    let start = Instant::now();
    let mut paths = vec![];
    while start.elapsed() < timeout {
        for e in sink.take() {
            if let Event::WorkspaceChanged { paths: p } = e {
                paths.extend(p);
            }
        }
        if !paths.is_empty() {
            // Dá tempo de chegar o restante do lote.
            std::thread::sleep(Duration::from_millis(600));
            for e in sink.take() {
                if let Event::WorkspaceChanged { paths: p } = e {
                    paths.extend(p);
                }
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    paths
}

fn request(name: &str) -> RequestFile {
    RequestFile {
        schema_version: 1,
        name: name.into(),
        method: HttpMethod::Get,
        url: "x".into(),
        query: vec![],
        headers: vec![],
        body: Body::None,
        settings: Settings::default(),
    }
}

/// V-17: alteração externa gera evento com o caminho relativo.
#[test]
fn external_change_emits_event() {
    let (_tmp, root) = common::empty_dir();
    let ws = Workspace::init(&root).unwrap();
    ws.collection_create("C", None).unwrap();
    let rel = ws.request_create("c", &request("X")).unwrap();
    std::thread::sleep(Duration::from_millis(1600)); // passa a janela das gravações do próprio app

    let sink = Arc::new(MemorySink::default());
    let _watcher = WorkspaceWatcher::start(&root, sink.clone()).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    std::fs::write(
        root.join(&rel),
        "{\"schemaVersion\":1,\"name\":\"Y\",\"method\":\"GET\",\"url\":\"y\"}\n",
    )
    .unwrap();
    let paths = changed_paths(&sink, Duration::from_secs(5));
    assert!(paths.contains(&rel), "{paths:?}");
}

#[test]
fn own_writes_and_git_folder_are_ignored() {
    let (_tmp, root) = common::empty_dir();
    let ws = Workspace::init(&root).unwrap();
    ws.collection_create("C", None).unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::thread::sleep(Duration::from_millis(1600));

    let sink = Arc::new(MemorySink::default());
    let _watcher = WorkspaceWatcher::start(&root, sink.clone()).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    ws.request_create("c", &request("Do app")).unwrap();
    ws.tree(); // ler a árvore não pode gerar eventos (senão vira um laço de recargas)
    std::fs::write(root.join(".git/index"), "x").unwrap();
    let paths = changed_paths(&sink, Duration::from_secs(2));
    assert!(paths.is_empty(), "{paths:?}");
}

/// V-19: uma rajada (como um git pull) chega agrupada.
#[test]
fn burst_of_changes_is_grouped() {
    let (_tmp, root) = common::empty_dir();
    let ws = Workspace::init(&root).unwrap();
    ws.collection_create("C", None).unwrap();
    std::thread::sleep(Duration::from_millis(1600));

    let sink = Arc::new(MemorySink::default());
    let _watcher = WorkspaceWatcher::start(&root, sink.clone()).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    for i in 0..50 {
        std::fs::write(
            root.join(format!("c/r{i}.json")),
            format!("{{\"schemaVersion\":1,\"name\":\"R{i}\",\"method\":\"GET\",\"url\":\"x\"}}\n"),
        )
        .unwrap();
    }
    let start = Instant::now();
    let mut batches = 0;
    let mut all = std::collections::BTreeSet::new();
    while start.elapsed() < Duration::from_secs(5) && all.len() < 50 {
        for e in sink.take() {
            if let Event::WorkspaceChanged { paths } = e {
                batches += 1;
                all.extend(paths.into_iter().filter(|p| p.ends_with(".json")));
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(all.len(), 50);
    assert!(
        batches <= 3,
        "esperava poucos eventos agrupados, vieram {batches}"
    );
}

#[test]
fn gitignore_change_reports_status() {
    let (_tmp, root) = common::empty_dir();
    Workspace::init(&root).unwrap();
    std::thread::sleep(Duration::from_millis(1600));
    let sink = Arc::new(MemorySink::default());
    let _watcher = WorkspaceWatcher::start(&root, sink.clone()).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    std::fs::write(root.join(".gitignore"), "node_modules\n").unwrap();
    let start = Instant::now();
    let mut status = None;
    while start.elapsed() < Duration::from_secs(5) && status.is_none() {
        for e in sink.take() {
            if let Event::Gitignore { ok } = e {
                status = Some(ok);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(status, Some(false));
}
