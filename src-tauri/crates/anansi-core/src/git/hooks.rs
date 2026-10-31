//! Hook `pre-commit` do anansi no repositório do workspace (alternativa F da
//! spec do Bearer Token).
//!
//! O push pelo app já recusa tokens Bearer em texto puro, mas um commit feito
//! pelo terminal ou pelo editor não passa por ele. O hook cobre esse caso:
//! bloqueia o commit se algum `.json` preparado tiver um valor `"Bearer <token>"`
//! (sem `{{...}}`) ou se um `environments/*.secrets.json` tiver sido forçado no
//! commit.
//!
//! - É instalado só quando não existe outro `pre-commit`: um hook do usuário
//!   nunca é sobrescrito (estado `custom`).
//! - Um hook do anansi de versão anterior é atualizado.
//! - Respeita o `core.hooksPath` (o caminho vem do `git rev-parse --git-path hooks`).
//! - Para pular uma vez: `git commit --no-verify`.

use super::git;
use crate::error::{AppError, AppResult};
use crate::workspace::fs_ops;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Linha que identifica o hook do anansi (e a versão do script).
pub const HOOK_MARKER: &str = "# anansi:pre-commit v1";

/// Situação do hook no repositório.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookState {
    /// O hook do anansi está instalado e atualizado.
    Installed,
    /// Já existe um `pre-commit` que não é do anansi; ele foi mantido.
    Custom,
    /// Sem repositório, sem git ou sem permissão para gravar.
    Unavailable,
}

/// Script do hook. Usa só `sh`, `git` e `grep -E`, que vêm no Git for Windows.
pub fn script() -> String {
    format!(
        r#"#!/bin/sh
{HOOK_MARKER}
# Instalado pelo anansi. Bloqueia commits com token Bearer em texto puro.
# Para pular uma vez: git commit --no-verify
# Se quiser usar um pre-commit próprio, substitua este arquivo: o anansi não sobrescreve hooks de terceiros.

pattern='"[[:space:]]*[Bb][Ee][Aa][Rr][Ee][Rr][[:space:]]+[^"{{[:space:]]+[[:space:]]*"'
files=$(git -c core.quotePath=false diff --cached --name-only --diff-filter=ACMR -- '*.json' '*.JSON')
[ -z "$files" ] && exit 0

secrets=$(printf '%s\n' "$files" | grep -E '(^|/)environments/[^/]*\.secrets\.json$')
bearer=$(printf '%s\n' "$files" | while IFS= read -r f; do
  if git show ":$f" 2>/dev/null | grep -Eq "$pattern"; then printf '  %s\n' "$f"; fi
done)

[ -z "$secrets$bearer" ] && exit 0

echo "anansi: commit bloqueado para nao vazar segredos." >&2
if [ -n "$bearer" ]; then
  echo "Token Bearer em texto puro em:" >&2
  printf '%s\n' "$bearer" >&2
  echo "Abra e salve esses requests no anansi (ou use \"Converter agora\" no Push): o token vai para o segredo bearer_token e o arquivo fica com Bearer {{{{bearer_token}}}}." >&2
fi
if [ -n "$secrets" ]; then
  echo "Arquivos de segredos locais no commit:" >&2
  printf '  %s\n' $secrets >&2
  echo "Tire do commit com: git rm --cached <arquivo>" >&2
fi
echo "Para pular esta verificacao uma vez: git commit --no-verify" >&2
exit 1
"#
    )
}

/// Pasta de hooks do repositório (respeita `core.hooksPath`).
fn hooks_dir(root: &Path) -> AppResult<PathBuf> {
    let path = git(root, &["rev-parse", "--git-path", "hooks"])?;
    Ok(root.join(path.trim()))
}

/// Instala ou atualiza o hook do anansi, sem tocar num hook de terceiros.
pub fn ensure_pre_commit(root: &Path) -> HookState {
    match try_ensure(root) {
        Ok(state) => state,
        Err(e) => {
            eprintln!("anansi: não foi possível instalar o hook pre-commit: {e}");
            HookState::Unavailable
        }
    }
}

fn try_ensure(root: &Path) -> AppResult<HookState> {
    let dir = hooks_dir(root)?;
    let file = dir.join("pre-commit");
    let wanted = script();
    match std::fs::read_to_string(&file) {
        Ok(current) if current == wanted => return Ok(HookState::Installed),
        Ok(current) if !current.contains("# anansi:pre-commit") => return Ok(HookState::Custom),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(AppError::io(&e, &file)),
    }
    std::fs::create_dir_all(&dir).map_err(|e| AppError::io(&e, &dir))?;
    fs_ops::write_atomic(&file, wanted.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| AppError::io(&e, &file))?;
    }
    Ok(HookState::Installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_has_marker_and_literal_braces() {
        let s = script();
        assert!(s.starts_with("#!/bin/sh\n# anansi:pre-commit v1\n"));
        assert!(s.contains("Bearer {{bearer_token}}"));
        assert!(s.contains(r#"[^"{[:space:]]+"#));
        assert!(!s.contains('\r'));
    }
}
