//! Operações de disco: escrita atômica, renomear e remover (Seção 4.3 da spec).

use crate::error::{AppError, AppResult};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Esperas entre as tentativas de renomear quando o destino está em uso (Windows).
const RETRY_DELAYS_MS: [u64; 5] = [50, 100, 200, 400, 800];

/// Janela em que eventos do watcher para caminhos gravados pelo próprio app são ignorados.
pub const SELF_WRITE_WINDOW: Duration = Duration::from_millis(1500);

fn self_writes() -> &'static Mutex<HashMap<PathBuf, Instant>> {
    static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, Instant>>> = OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

/// Registra que o app acabou de alterar `path` (arquivo ou pasta) (Seção 12).
pub fn record_self_write(path: &Path) {
    let mut map = self_writes().lock().unwrap();
    let now = Instant::now();
    map.retain(|_, t| now.duration_since(*t) < SELF_WRITE_WINDOW * 4);
    map.insert(path.to_path_buf(), now);
}

/// `true` se o app alterou `path` (ou uma pasta que o contém, ou um item direto dentro dele,
/// já que criar um arquivo também muda a pasta) há menos de `SELF_WRITE_WINDOW`.
pub fn is_recent_self_write(path: &Path) -> bool {
    let map = self_writes().lock().unwrap();
    let now = Instant::now();
    map.iter().any(|(p, t)| {
        now.duration_since(*t) < SELF_WRITE_WINDOW
            && (path == p || path.starts_with(p) || p.parent() == Some(path))
    })
}

/// Nome do arquivo temporário da escrita atômica: `.<nome>.tmp`, na mesma pasta.
pub fn temp_path_for(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.tmp"))
}

/// Grava `content` em `path` de forma atômica.
///
/// Retorna `false` sem tocar no disco quando o conteúdo atual já é igual
/// (salvar sem mudanças não altera o arquivo, RNF-04).
pub fn write_atomic(path: &Path, content: &[u8]) -> AppResult<bool> {
    if let Ok(current) = std::fs::read(path) {
        if current == content {
            return Ok(false);
        }
    }
    let tmp = temp_path_for(path);
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(content)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(AppError::io(&e, path));
    }
    record_self_write(path);
    record_self_write(&tmp);
    if let Err(err) = rename_with_retry(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err);
    }
    record_self_write(path);
    Ok(true)
}

/// Indica se o erro é transitório: outro processo está com o arquivo aberto.
fn is_in_use(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::PermissionDenied {
        return true;
    }
    // ERROR_ACCESS_DENIED (5), ERROR_SHARING_VIOLATION (32), ERROR_LOCK_VIOLATION (33)
    cfg!(windows) && matches!(err.raw_os_error(), Some(5) | Some(32) | Some(33))
}

/// Renomeia `from` para `to` (substituindo `to` se for arquivo), com novas
/// tentativas quando o destino está em uso por outro programa.
pub fn rename_with_retry(from: &Path, to: &Path) -> AppResult<()> {
    record_self_write(from);
    record_self_write(to);
    rename_with(from, to, &RETRY_DELAYS_MS, |a, b| std::fs::rename(a, b))
}

fn rename_with(
    from: &Path,
    to: &Path,
    delays: &[u64],
    mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> AppResult<()> {
    let mut attempt = 0;
    loop {
        match rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if is_in_use(&e) && attempt < delays.len() => {
                sleep(Duration::from_millis(delays[attempt]));
                attempt += 1;
            }
            Err(e) if is_in_use(&e) => {
                return Err(AppError::IoError(format!(
                    "Não foi possível gravar {}: o arquivo está em uso por outro programa. Feche-o e tente de novo.",
                    to.display()
                )))
            }
            Err(e) => return Err(AppError::io(&e, to)),
        }
    }
}

/// Cria um diretório (e os pais) com um `.gitkeep`, para que o git versione a pasta vazia.
pub fn create_dir_with_gitkeep(dir: &Path) -> AppResult<()> {
    record_self_write(dir);
    std::fs::create_dir_all(dir).map_err(|e| AppError::io(&e, dir))?;
    let keep = dir.join(".gitkeep");
    if !keep.exists() {
        std::fs::write(&keep, b"").map_err(|e| AppError::io(&e, &keep))?;
    }
    Ok(())
}

/// Cria um diretório (sem `.gitkeep`).
pub fn create_dir(dir: &Path) -> AppResult<()> {
    record_self_write(dir);
    std::fs::create_dir(dir).map_err(|e| AppError::io(&e, dir))
}

pub fn remove_file_if_exists(path: &Path) -> AppResult<()> {
    record_self_write(path);
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io(&e, path)),
    }
}

pub fn remove_dir_all(path: &Path) -> AppResult<()> {
    record_self_write(path);
    std::fs::remove_dir_all(path).map_err(|e| AppError::io(&e, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn writes_and_skips_identical_content() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.json");
        assert!(write_atomic(&file, b"um\n").unwrap());
        let modified = std::fs::metadata(&file).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        assert!(
            !write_atomic(&file, b"um\n").unwrap(),
            "conteúdo igual não regrava"
        );
        assert_eq!(
            std::fs::metadata(&file).unwrap().modified().unwrap(),
            modified
        );
        assert!(write_atomic(&file, b"dois\n").unwrap());
        assert_eq!(std::fs::read(&file).unwrap(), b"dois\n");
        assert!(!temp_path_for(&file).exists(), "o .tmp não sobra");
    }

    #[test]
    fn temp_file_is_hidden_and_in_same_folder() {
        assert_eq!(
            temp_path_for(Path::new("/w/c/x.json")),
            Path::new("/w/c/.x.json.tmp")
        );
    }

    #[test]
    fn retries_while_file_is_in_use_then_succeeds() {
        let calls = Cell::new(0);
        let result = rename_with(Path::new("a"), Path::new("b"), &[1, 1, 1], |_, _| {
            calls.set(calls.get() + 1);
            if calls.get() < 3 {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            } else {
                Ok(())
            }
        });
        assert!(result.is_ok());
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn gives_up_with_clear_message_when_file_stays_in_use() {
        let calls = Cell::new(0);
        let err = rename_with(Path::new("a"), Path::new("b"), &[1, 1], |_, _| {
            calls.set(calls.get() + 1);
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        })
        .unwrap_err();
        assert_eq!(calls.get(), 3, "uma tentativa inicial e duas novas");
        assert_eq!(err.code(), "IO_ERROR");
        assert!(err.to_string().contains("em uso por outro programa"));
    }

    #[test]
    fn other_errors_are_not_retried() {
        let calls = Cell::new(0);
        let err = rename_with(Path::new("a"), Path::new("b"), &[1, 1], |_, _| {
            calls.set(calls.get() + 1);
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        })
        .unwrap_err();
        assert_eq!(calls.get(), 1);
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn records_self_writes_including_folder_contents() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.json");
        assert!(!is_recent_self_write(&file));
        write_atomic(&file, b"1").unwrap();
        assert!(is_recent_self_write(&file));
        let folder = dir.path().join("pasta");
        create_dir_with_gitkeep(&folder).unwrap();
        assert!(
            is_recent_self_write(&folder.join(".gitkeep")),
            "o que está dentro da pasta criada também conta"
        );
        assert!(!is_recent_self_write(&dir.path().join("outro.json")));
    }

    #[test]
    fn gitkeep_is_created() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("a/b");
        create_dir_with_gitkeep(&folder).unwrap();
        assert!(folder.join(".gitkeep").exists());
    }

    /// No Windows, um handle aberto sem FILE_SHARE_DELETE impede a substituição
    /// do arquivo; a escrita precisa falhar com mensagem clara e sem corromper o original.
    #[cfg(windows)]
    #[test]
    fn locked_file_keeps_original_content() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.json");
        std::fs::write(&file, b"original").unwrap();
        let _lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&file)
            .unwrap();
        let err = write_atomic(&file, b"novo").unwrap_err();
        assert_eq!(err.code(), "IO_ERROR");
        drop(_lock);
        assert_eq!(std::fs::read(&file).unwrap(), b"original");
        assert!(!temp_path_for(&file).exists());
    }
}
