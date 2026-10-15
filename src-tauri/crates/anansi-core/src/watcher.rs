//! File watcher do workspace (RF-17, Seção 12 da spec).
//!
//! Avisa o frontend quando arquivos mudam por fora do app (git pull, editor),
//! ignorando `.git/`, os temporários da escrita atômica e o que o próprio app
//! acabou de gravar. Um `git pull` com muitos arquivos vira um único evento.

use crate::env::gitignore;
use crate::events::{Event, EventSink};
use crate::workspace::{fs_ops, paths};
use notify_debouncer_full::notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub const DEBOUNCE: Duration = Duration::from_millis(300);

/// Mantém o watcher vivo; ao ser descartado, para de observar.
pub struct WorkspaceWatcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

/// Decide se um caminho alterado interessa ao frontend.
pub fn is_relevant(root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    let mut components = rel.components();
    if components
        .next()
        .map(|c| c.as_os_str() == ".git")
        .unwrap_or(true)
    {
        return false;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.ends_with(".tmp") {
        return false;
    }
    !fs_ops::is_recent_self_write(path)
}

/// Converte os caminhos de um lote de eventos nos eventos do frontend.
pub fn events_for(root: &Path, changed: impl IntoIterator<Item = PathBuf>) -> Vec<Event> {
    let mut rel_paths = BTreeSet::new();
    let mut gitignore_changed = false;
    for path in changed {
        if !is_relevant(root, &path) {
            continue;
        }
        if let Ok(rel) = paths::to_relative(root, &path) {
            if rel == ".gitignore" {
                gitignore_changed = true;
            }
            rel_paths.insert(rel);
        }
    }
    let mut events = Vec::new();
    if gitignore_changed {
        events.push(Event::Gitignore {
            ok: gitignore::status(root),
        });
    }
    if !rel_paths.is_empty() {
        events.push(Event::WorkspaceChanged {
            paths: rel_paths.into_iter().collect(),
        });
    }
    events
}

impl WorkspaceWatcher {
    /// Começa a observar `root` (recursivo, com debounce de 300 ms).
    pub fn start(root: &Path, sink: Arc<dyn EventSink>) -> Result<Self, String> {
        let root_for_handler = root.to_path_buf();
        let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            // Leituras (abrir/fechar sem escrita) não são mudanças; o próprio scanner geraria eventos.
            let changed = events
                .into_iter()
                .filter(|e| !matches!(e.event.kind, EventKind::Access(_)))
                .flat_map(|e| e.event.paths.clone());
            for event in events_for(&root_for_handler, changed) {
                sink.emit(event);
            }
        })
        .map_err(|e| format!("Não foi possível observar o workspace: {e}"))?;
        debouncer
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| format!("Não foi possível observar o workspace: {e}"))?;
        Ok(WorkspaceWatcher {
            _debouncer: debouncer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_git_temp_files_and_paths_outside_root() {
        let root = Path::new("/w");
        assert!(!is_relevant(root, Path::new("/w/.git/index")));
        assert!(!is_relevant(root, Path::new("/w/c/.x.json.tmp")));
        assert!(!is_relevant(root, Path::new("/outro/x.json")));
        assert!(!is_relevant(root, Path::new("/w")));
        assert!(is_relevant(root, Path::new("/w/c/x.json")));
    }

    #[test]
    fn groups_paths_and_reports_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join(".gitignore"), "environments/*.secrets.json\n").unwrap();
        let events = events_for(
            root,
            vec![
                root.join("c/b.json"),
                root.join("c/a.json"),
                root.join("c/a.json"),
                root.join(".gitignore"),
            ],
        );
        assert_eq!(
            events,
            vec![
                Event::Gitignore { ok: true },
                Event::WorkspaceChanged {
                    paths: vec![".gitignore".into(), "c/a.json".into(), "c/b.json".into()]
                }
            ]
        );
    }
}
