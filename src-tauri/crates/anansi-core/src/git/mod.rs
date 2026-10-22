//! Sincronização do workspace com o git: os botões de pull e push da barra superior.
//!
//! Usa o `git` instalado na máquina, e não uma lib como a libgit2: assim valem as
//! credenciais que o usuário já tem (Git Credential Manager, agente SSH), o
//! `.gitconfig` e os hooks do repositório. Os comandos rodam na raiz do workspace
//! com o pathspec `.`, então, se o workspace for uma subpasta do repositório,
//! só ela entra no commit.
//!
//! - Pull: `git pull --rebase --autostash` na branch atual. Se der conflito, o
//!   rebase é abortado e nada muda no disco.
//! - Push: `git add --all -- .`, `git commit -- .`, pull (como acima) e `git push`.

use crate::env::gitignore;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// Se dá para usar o git no workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GitAvailability {
    Ready,
    NotARepo,
    GitMissing,
}

/// Situação do repositório, para os botões da barra superior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub availability: GitAvailability,
    /// `None` com HEAD destacado (fora de qualquer branch).
    pub branch: Option<String>,
    /// Ex.: `origin/main`. `None` se a branch ainda não existe no remote.
    pub upstream: Option<String>,
    /// Arquivos alterados, novos ou removidos dentro do workspace.
    pub changes: usize,
    /// Commits locais que não estão no remote (desde o último fetch).
    pub ahead: usize,
    /// Commits do remote que não estão aqui (desde o último fetch).
    pub behind: usize,
}

impl GitStatus {
    fn unavailable(availability: GitAvailability) -> Self {
        GitStatus {
            availability,
            branch: None,
            upstream: None,
            changes: 0,
            ahead: 0,
            behind: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullResult {
    /// `true` se vieram commits novos do remote.
    pub updated: bool,
    pub status: GitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushResult {
    /// `true` se as alterações do workspace viraram um commit.
    pub committed: bool,
    /// `true` se algum commit foi enviado ao remote.
    pub pushed: bool,
    pub status: GitStatus,
}

fn git_error(msg: impl Into<String>) -> AppError {
    AppError::GitError(msg.into())
}

fn command(root: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        // Sem terminal para responder: falha em vez de ficar esperando usuário e senha.
        // O Git Credential Manager continua abrindo a própria janela de login.
        .env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn output(root: &Path, args: &[&str]) -> AppResult<Output> {
    command(root, args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            return git_error(
                "O git não foi encontrado. Instale o Git for Windows e reabra o anansi.",
            );
        }
        git_error(format!("Não foi possível executar o git: {e}"))
    })
}

/// Roda o git e devolve o stdout; se falhar, o erro traz a saída do próprio git.
fn git(root: &Path, args: &[&str]) -> AppResult<String> {
    let out = output(root, args)?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    Err(git_error(format!("git {} falhou: {detail}", args[0])))
}

/// Situação do repositório. Nunca falha por falta de git ou de repositório:
/// isso volta em `availability`, para a interface só desabilitar os botões.
pub fn status(root: &Path) -> AppResult<GitStatus> {
    match output(root, &["rev-parse", "--is-inside-work-tree"]) {
        Err(_) => return Ok(GitStatus::unavailable(GitAvailability::GitMissing)),
        Ok(out) if !out.status.success() => {
            return Ok(GitStatus::unavailable(GitAvailability::NotARepo))
        }
        Ok(_) => {}
    }
    let porcelain = git(
        root,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
            "--",
            ".",
        ],
    )?;
    Ok(parse_status(&porcelain))
}

/// Interpreta o `git status --porcelain=v2 --branch`.
fn parse_status(porcelain: &str) -> GitStatus {
    let mut status = GitStatus::unavailable(GitAvailability::Ready);
    for line in porcelain.lines() {
        let Some(header) = line.strip_prefix("# ") else {
            if !line.trim().is_empty() {
                status.changes += 1;
            }
            continue;
        };
        let (key, value) = header.split_once(' ').unwrap_or((header, ""));
        match key {
            "branch.head" if value != "(detached)" => status.branch = Some(value.to_string()),
            "branch.upstream" => status.upstream = Some(value.to_string()),
            "branch.ab" => {
                for part in value.split_whitespace() {
                    if let Some(n) = part.strip_prefix('+') {
                        status.ahead = n.parse().unwrap_or(0);
                    } else if let Some(n) = part.strip_prefix('-') {
                        status.behind = n.parse().unwrap_or(0);
                    }
                }
            }
            _ => {}
        }
    }
    status
}

/// Status de um repositório pronto e numa branch; senão, um erro explicando.
fn ready_status(root: &Path) -> AppResult<(GitStatus, String)> {
    let st = status(root)?;
    match st.availability {
        GitAvailability::GitMissing => {
            return Err(git_error(
                "O git não foi encontrado. Instale o Git for Windows e reabra o anansi.",
            ))
        }
        GitAvailability::NotARepo => {
            return Err(git_error(
                "A pasta do workspace não está num repositório git. Clone o repositório de collections e abra a pasta dele.",
            ))
        }
        GitAvailability::Ready => {}
    }
    let Some(branch) = st.branch.clone() else {
        return Err(git_error(
            "O repositório não está em nenhuma branch (HEAD destacado). Faça checkout da branch pelo terminal.",
        ));
    };
    Ok((st, branch))
}

fn head(root: &Path) -> Option<String> {
    git(root, &["rev-parse", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
}

fn rebase_in_progress(root: &Path) -> bool {
    ["rebase-merge", "rebase-apply"].iter().any(|dir| {
        git(root, &["rev-parse", "--git-path", dir])
            .map(|p| root.join(p.trim()).exists())
            .unwrap_or(false)
    })
}

/// `git pull --rebase --autostash`. Em conflito, aborta o rebase (o disco volta
/// a como estava, inclusive as alterações não commitadas) e explica.
fn pull_rebase(root: &Path, committed: bool) -> AppResult<()> {
    let Err(err) = git(root, &["pull", "--rebase", "--autostash"]) else {
        return Ok(());
    };
    if !rebase_in_progress(root) {
        return Err(err);
    }
    let files = git(root, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
    let files: Vec<&str> = files.lines().filter(|l| !l.is_empty()).collect();
    let _ = git(root, &["rebase", "--abort"]);
    let local = if committed {
        "O seu commit foi feito, mas ficou só na sua máquina."
    } else {
        "Nada foi alterado na sua máquina."
    };
    let which = if files.is_empty() {
        String::new()
    } else {
        format!(" ({})", files.join(", "))
    };
    Err(git_error(format!(
        "As alterações do remote conflitam com as suas{which}. {local} Resolva o conflito pelo terminal ou editor (git pull --rebase) e tente de novo."
    )))
}

/// Traz as alterações do remote para a branch atual.
pub fn pull(root: &Path) -> AppResult<PullResult> {
    let (st, branch) = ready_status(root)?;
    if st.upstream.is_none() {
        return Err(git_error(format!(
            "A branch {branch} ainda não existe no remote, então não há o que puxar."
        )));
    }
    let before = head(root);
    pull_rebase(root, false)?;
    Ok(PullResult {
        updated: head(root) != before,
        status: status(root)?,
    })
}

/// Remote para o primeiro push de uma branch: `origin`, ou o único que houver.
fn default_remote(root: &Path) -> AppResult<String> {
    let remotes = git(root, &["remote"])?;
    let remotes: Vec<&str> = remotes
        .lines()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .collect();
    if remotes.contains(&"origin") {
        return Ok("origin".into());
    }
    match remotes.as_slice() {
        [only] => Ok(only.to_string()),
        [] => Err(git_error("O repositório não tem remote configurado.")),
        _ => Err(git_error(
            "O repositório tem vários remotes e nenhum se chama origin. Faça o primeiro push pelo terminal.",
        )),
    }
}

/// Commita tudo o que mudou no workspace (se houver), traz o remote e envia.
///
/// `message` é obrigatório só quando há alterações para commitar.
pub fn push(root: &Path, message: Option<&str>) -> AppResult<PushResult> {
    let (st, branch) = ready_status(root)?;
    let mut committed = false;
    if st.changes > 0 {
        let message = message
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .ok_or_else(|| AppError::InvalidName("Informe o título do commit.".into()))?;
        if !gitignore::status(root) {
            return Err(AppError::GitignoreNotConfigured(
                "O .gitignore não protege os arquivos de segredos. Use \"Corrigir\" na barra superior antes de enviar."
                    .into(),
            ));
        }
        git(root, &["add", "--all", "--", "."])?;
        git(root, &["commit", "-m", message, "--", "."])?;
        committed = true;
    }
    let pushed = match st.upstream {
        Some(_) => {
            pull_rebase(root, committed)?;
            let ahead = status(root)?.ahead;
            if ahead > 0 {
                git(root, &["push"])?;
            }
            ahead > 0
        }
        None => {
            let remote = default_remote(root)?;
            git(root, &["push", "--set-upstream", &remote, &branch])?;
            true
        }
    };
    Ok(PushResult {
        committed,
        pushed,
        status: status(root)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_branch_upstream_divergence_and_changes() {
        let porcelain = "\
# branch.oid 4decfa5
# branch.head main
# branch.upstream origin/main
# branch.ab +2 -3
1 .M N... 100644 100644 100644 abc abc store-api/a.json
? store-api/novo.json
";
        assert_eq!(
            parse_status(porcelain),
            GitStatus {
                availability: GitAvailability::Ready,
                branch: Some("main".into()),
                upstream: Some("origin/main".into()),
                changes: 2,
                ahead: 2,
                behind: 3,
            }
        );
    }

    #[test]
    fn detached_head_and_branch_without_upstream() {
        let detached = parse_status("# branch.oid abc\n# branch.head (detached)\n");
        assert_eq!(detached.branch, None);
        let local = parse_status("# branch.oid (initial)\n# branch.head nova\n");
        assert_eq!(local.branch.as_deref(), Some("nova"));
        assert_eq!(local.upstream, None);
        assert_eq!((local.ahead, local.behind, local.changes), (0, 0, 0));
    }
}
