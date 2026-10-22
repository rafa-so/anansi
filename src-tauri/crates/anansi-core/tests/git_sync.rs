//! Pull e push pela barra superior, com repositórios git de verdade: um remote
//! "bare" e dois clones, como dois colegas usando as mesmas collections.

mod common;

use anansi_core::git::{self, GitAvailability};
use anansi_core::Workspace;
use common::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn run(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn configure(repo: &Path) {
    run(repo, &["config", "user.name", "Teste"]);
    run(repo, &["config", "user.email", "teste@example.com"]);
    run(repo, &["config", "commit.gpgsign", "false"]);
}

/// Remote com um workspace inicial na `main` e dois clones (`alice` e `bruno`).
struct Team {
    _tmp: TempDir,
    remote: PathBuf,
    alice: PathBuf,
    bruno: PathBuf,
}

fn team() -> Team {
    let (tmp, base) = empty_dir();
    let remote = base.join("remote.git");
    std::fs::create_dir(&remote).unwrap();
    run(&remote, &["init", "--bare", "--initial-branch=main"]);
    let alice = base.join("alice");
    run(&base, &["clone", "-q", remote.to_str().unwrap(), "alice"]);
    configure(&alice);
    Workspace::init(&alice).unwrap();
    run(&alice, &["add", "--all"]);
    run(&alice, &["commit", "-q", "-m", "workspace inicial"]);
    run(&alice, &["push", "-q", "-u", "origin", "main"]);
    let bruno = base.join("bruno");
    run(&base, &["clone", "-q", remote.to_str().unwrap(), "bruno"]);
    configure(&bruno);
    Team {
        _tmp: tmp,
        remote,
        alice,
        bruno,
    }
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn last_message(repo: &Path) -> String {
    run(repo, &["log", "-1", "--format=%s"]).trim().to_string()
}

#[test]
fn status_counts_changes_and_reports_branch() {
    let t = team();
    let st = git::status(&t.alice).unwrap();
    assert_eq!(st.availability, GitAvailability::Ready);
    assert_eq!(st.branch.as_deref(), Some("main"));
    assert_eq!(st.upstream.as_deref(), Some("origin/main"));
    assert_eq!(st.changes, 0);

    write(&t.alice, "store-api/collection.json", "{}");
    write(&t.alice, "store-api/products/a.json", "{}");
    assert_eq!(git::status(&t.alice).unwrap().changes, 2);
}

#[test]
fn status_outside_a_repository_is_not_an_error() {
    let (_tmp, root) = empty_dir();
    let st = git::status(&root).unwrap();
    assert_eq!(st.availability, GitAvailability::NotARepo);
    let err = git::pull(&root).unwrap_err();
    assert_eq!(err.code(), "GIT_ERROR");
}

#[test]
fn push_commits_everything_and_pull_brings_it_to_the_other_clone() {
    let t = team();
    write(
        &t.alice,
        "store-api/collection.json",
        "{\"name\":\"Store\"}",
    );
    let pushed = git::push(&t.alice, Some("  adiciona store-api  ")).unwrap();
    assert!(pushed.committed && pushed.pushed);
    assert_eq!((pushed.status.changes, pushed.status.ahead), (0, 0));
    assert_eq!(last_message(&t.remote), "adiciona store-api");

    let pulled = git::pull(&t.bruno).unwrap();
    assert!(pulled.updated);
    assert!(exists(&t.bruno, "store-api/collection.json"));

    let again = git::pull(&t.bruno).unwrap();
    assert!(!again.updated);
}

#[test]
fn push_requires_a_message_only_when_there_are_changes() {
    let t = team();
    write(&t.alice, "a.json", "{}");
    let err = git::push(&t.alice, Some("   ")).unwrap_err();
    assert_eq!(err.code(), "INVALID_NAME");
    assert_eq!(
        git::status(&t.alice).unwrap().changes,
        1,
        "nada foi commitado"
    );

    run(&t.alice, &["add", "--all"]);
    run(&t.alice, &["commit", "-q", "-m", "commit pelo terminal"]);
    let res = git::push(&t.alice, None).unwrap();
    assert!(!res.committed && res.pushed);
    assert_eq!(last_message(&t.remote), "commit pelo terminal");

    let nothing = git::push(&t.alice, None).unwrap();
    assert!(!nothing.committed && !nothing.pushed);
}

#[test]
fn push_includes_removed_files() {
    let t = team();
    std::fs::remove_file(t.alice.join(".gitattributes")).unwrap();
    git::push(&t.alice, Some("remove gitattributes")).unwrap();
    git::pull(&t.bruno).unwrap();
    assert!(!exists(&t.bruno, ".gitattributes"));
}

#[test]
fn push_rebases_over_commits_from_the_team() {
    let t = team();
    write(&t.bruno, "billing/collection.json", "{}");
    git::push(&t.bruno, Some("billing")).unwrap();

    write(&t.alice, "store/collection.json", "{}");
    let res = git::push(&t.alice, Some("store")).unwrap();
    assert!(res.pushed);
    assert!(exists(&t.alice, "billing/collection.json"));
    let log = run(&t.remote, &["log", "--format=%s"]);
    assert_eq!(
        log.lines().take(2).collect::<Vec<_>>(),
        ["store", "billing"]
    );
}

#[test]
fn pull_keeps_uncommitted_changes() {
    let t = team();
    write(&t.bruno, "billing/collection.json", "{}");
    git::push(&t.bruno, Some("billing")).unwrap();

    write(&t.alice, "rascunho.json", "{\"wip\":true}");
    let res = git::pull(&t.alice).unwrap();
    assert!(res.updated);
    assert_eq!(read(&t.alice, "rascunho.json"), "{\"wip\":true}");
    assert_eq!(res.status.changes, 1);
}

#[test]
fn conflict_aborts_the_rebase_and_keeps_the_local_commit() {
    let t = team();
    write(&t.bruno, "workspace.json", "{\"name\":\"Bruno\"}\n");
    git::push(&t.bruno, Some("nome do bruno")).unwrap();

    write(&t.alice, "workspace.json", "{\"name\":\"Alice\"}\n");
    let err = git::push(&t.alice, Some("nome da alice")).unwrap_err();
    assert_eq!(err.code(), "GIT_ERROR");
    assert!(err.to_string().contains("workspace.json"), "{err}");
    assert!(err.to_string().contains("ficou só na sua máquina"), "{err}");
    // O rebase foi abortado: o arquivo e o commit locais continuam como estavam.
    assert_eq!(read(&t.alice, "workspace.json"), "{\"name\":\"Alice\"}\n");
    assert_eq!(last_message(&t.alice), "nome da alice");
    assert_eq!(last_message(&t.remote), "nome do bruno");
    let st = git::status(&t.alice).unwrap();
    assert_eq!((st.ahead, st.changes), (1, 0));
}

#[test]
fn refuses_to_commit_when_gitignore_does_not_protect_secrets() {
    let t = team();
    std::fs::write(t.alice.join(".gitignore"), "").unwrap();
    write(
        &t.alice,
        "environments/hml.secrets.json",
        "{\"token\":\"x\"}",
    );
    let err = git::push(&t.alice, Some("vaza segredo")).unwrap_err();
    assert_eq!(err.code(), "GITIGNORE_NOT_CONFIGURED");
    assert_eq!(last_message(&t.alice), "workspace inicial");
}

#[test]
fn first_push_of_a_new_branch_sets_the_upstream() {
    let t = team();
    run(&t.alice, &["checkout", "-q", "-b", "nova"]);
    assert!(
        git::pull(&t.alice).is_err(),
        "sem upstream não há o que puxar"
    );
    write(&t.alice, "a.json", "{}");
    let res = git::push(&t.alice, Some("branch nova")).unwrap();
    assert!(res.pushed);
    assert_eq!(res.status.upstream.as_deref(), Some("origin/nova"));
}

#[test]
fn workspace_in_a_subfolder_only_commits_the_subfolder() {
    let t = team();
    let ws = t.alice.join("colecoes");
    std::fs::create_dir(&ws).unwrap();
    Workspace::init(&ws).unwrap();
    write(&t.alice, "fora.txt", "fora do workspace");
    // workspace.json, .gitignore, .gitattributes e environments/.gitkeep
    assert_eq!(git::status(&ws).unwrap().changes, 4);
    git::push(&ws, Some("só a subpasta")).unwrap();
    git::pull(&t.bruno).unwrap();
    assert!(exists(&t.bruno, "colecoes/workspace.json"));
    assert!(!exists(&t.bruno, "fora.txt"));
    assert!(exists(&t.alice, "fora.txt"));
}
