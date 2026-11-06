//! Workspace aberto: árvore e CRUD de collections, pastas e requests
//! (RF-01 a RF-07, RF-18, Seção 4 da spec).

pub mod fs_ops;
pub mod paths;
pub mod scanner;
pub mod slug;

use crate::env;
use crate::env::bearer::{self, BearerOutcome};
use crate::error::{AppError, AppResult};
use crate::model::schema::load;
use crate::model::*;
use paths::{check_length, file_name_rel, join_rel, parent_rel, resolve};
use scanner::{is_request_file_name, scan_children, scan_collection, COLLECTION_FILE};
use serde::Serialize;
use slug::{existing_names, slugify, unique_name};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const WORKSPACE_FILE: &str = "workspace.json";
pub const GITATTRIBUTES_CONTENT: &str = "* text=auto eol=lf\n";

const BODY_SUFFIXES: [&str; 2] = [".body.json", ".body.txt"];

/// Resultado de criar ou salvar um request: o caminho, o request como foi
/// gravado (com `{{bearer_token}}` no lugar do token) e o que aconteceu com o segredo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRequest {
    pub path: String,
    pub request: RequestFile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer: Option<BearerOutcome>,
}

/// Resultado de mover um request: o novo caminho e se o nome do arquivo mudou
/// por já existir outro igual no destino.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedRequest {
    pub path: String,
    pub renamed: bool,
}

/// Workspace aberto. Todos os caminhos recebidos são relativos à raiz, com `/`.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
}

fn invalid_name(msg: &str) -> AppError {
    AppError::InvalidName(msg.to_string())
}

fn clean_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid_name("Informe um nome."));
    }
    if name.chars().any(char::is_control) {
        return Err(invalid_name("O nome não pode ter caracteres de controle."));
    }
    Ok(name.to_string())
}

fn clean_description(description: Option<&str>) -> Option<String> {
    description
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(str::to_string)
}

/// Nomes ocupados num diretório, sem os do próprio item (para renomear).
fn taken_except(dir: &Path, own: &[String]) -> HashSet<String> {
    let mut taken = existing_names(dir);
    for name in own {
        taken.remove(&name.to_lowercase());
    }
    taken
}

impl Workspace {
    /// Abre uma pasta que já é workspace (RF-01).
    pub fn open(path: &Path) -> AppResult<Workspace> {
        let root = canonical_dir(path)?;
        let file = root.join(WORKSPACE_FILE);
        if !file.is_file() {
            return Err(AppError::NotAWorkspace(format!(
                "A pasta {} não é um workspace do anansi (falta o workspace.json).",
                root.display()
            )));
        }
        let ws = Workspace { root };
        ws.read_workspace_file()?;
        Ok(ws)
    }

    /// Inicializa a estrutura mínima (Seção 4.2) sem sobrescrever o que já existe.
    pub fn init(path: &Path) -> AppResult<Workspace> {
        let root = canonical_dir(path)?;
        let file = root.join(WORKSPACE_FILE);
        if !file.exists() {
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Workspace".into());
            let content = WorkspaceFile {
                schema_version: WORKSPACE_SCHEMA_VERSION,
                name,
            };
            fs_ops::write_atomic(&file, &to_file_bytes(&content))?;
        }
        fs_ops::create_dir_with_gitkeep(&root.join(env::ENVIRONMENTS_DIR))?;
        let gitignore = root.join(".gitignore");
        if gitignore.exists() {
            env::gitignore::fix(&root)?;
        } else {
            fs_ops::write_atomic(&gitignore, env::gitignore::initial_content().as_bytes())?;
        }
        let gitattributes = root.join(".gitattributes");
        if !gitattributes.exists() {
            fs_ops::write_atomic(&gitattributes, GITATTRIBUTES_CONTENT.as_bytes())?;
        }
        Workspace::open(&root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn read_workspace_file(&self) -> AppResult<WorkspaceFile> {
        let file = self.root.join(WORKSPACE_FILE);
        let text = std::fs::read_to_string(&file).map_err(|e| AppError::io(&e, &file))?;
        load::<WorkspaceFile>(&text, WORKSPACE_SCHEMA_VERSION, &[])
            .map(|l| l.value)
            .map_err(|e| match e {
                AppError::InvalidFile(m) => AppError::InvalidFile(format!("workspace.json: {m}")),
                other => other,
            })
    }

    pub fn info(&self) -> AppResult<WorkspaceInfo> {
        Ok(WorkspaceInfo {
            path: self.root.display().to_string(),
            name: self.read_workspace_file()?.name,
            environments: env::list(&self.root),
            gitignore_ok: env::gitignore::status(&self.root),
        })
    }

    /// Árvore completa, ordenada (RN-11).
    pub fn tree(&self) -> Vec<TreeNode> {
        scanner::scan(&self.root)
    }

    // ----- validação de tipos de item -----

    fn collection_dir(&self, rel: &str) -> AppResult<PathBuf> {
        let dir = resolve(&self.root, rel)?;
        if rel.is_empty() || rel.contains('/') || !dir.join(COLLECTION_FILE).is_file() {
            return Err(AppError::not_found(rel));
        }
        Ok(dir)
    }

    /// Diretório que pode receber pastas e requests: uma collection ou uma pasta dentro dela.
    fn container_dir(&self, rel: &str) -> AppResult<PathBuf> {
        let collection = rel.split('/').next().unwrap_or("");
        self.collection_dir(collection)?;
        let dir = resolve(&self.root, rel)?;
        if !dir.is_dir() {
            return Err(AppError::not_found(rel));
        }
        Ok(dir)
    }

    fn folder_dir(&self, rel: &str) -> AppResult<PathBuf> {
        if !rel.contains('/') {
            return Err(AppError::not_found(rel));
        }
        self.container_dir(rel)
    }

    /// Caminho de um request. O arquivo pode não existir (aba "removido do disco").
    fn request_location(&self, rel: &str) -> AppResult<(PathBuf, String)> {
        let parent = parent_rel(rel);
        let file = file_name_rel(rel);
        if parent.is_empty() || !is_request_file_name(file, parent) {
            return Err(AppError::not_found(rel));
        }
        let dir = self.container_dir(parent)?;
        let stem = file[..file.len() - ".json".len()].to_string();
        Ok((dir, stem))
    }

    fn node_of_container(&self, rel: &str) -> AppResult<TreeNode> {
        if rel.contains('/') {
            let dir = self.folder_dir(rel)?;
            Ok(TreeNode::Folder {
                path: rel.into(),
                name: file_name_rel(rel).into(),
                children: scan_children(&dir, rel),
            })
        } else {
            Ok(scan_collection(&self.collection_dir(rel)?, rel))
        }
    }

    /// Quantos requests serão apagados ao remover o item (texto da confirmação).
    pub fn request_count(&self, rel: &str) -> AppResult<usize> {
        if rel.ends_with(".json") {
            self.request_location(rel)?;
            return Ok(1);
        }
        Ok(self.node_of_container(rel)?.request_count())
    }

    // ----- collections -----

    pub fn collection_create(&self, name: &str, description: Option<&str>) -> AppResult<TreeNode> {
        let name = clean_name(name)?;
        let dir_name = unique_name(&existing_names(&self.root), &slugify(&name), "", &[]);
        check_length(&dir_name)?;
        let dir = self.root.join(&dir_name);
        fs_ops::create_dir(&dir)?;
        let file = CollectionFile {
            schema_version: COLLECTION_SCHEMA_VERSION,
            name,
            description: clean_description(description),
        };
        fs_ops::write_atomic(&dir.join(COLLECTION_FILE), &to_file_bytes(&file))?;
        self.node_of_container(&dir_name)
    }

    pub fn collection_update(
        &self,
        rel: &str,
        name: &str,
        description: Option<&str>,
    ) -> AppResult<TreeNode> {
        let mut dir = self.collection_dir(rel)?;
        let name = clean_name(name)?;
        let mut rel = rel.to_string();
        let slug = slugify(&name);
        if !slug.eq_ignore_ascii_case(&rel) {
            let new_name = unique_name(&taken_except(&self.root, &[rel.clone()]), &slug, "", &[]);
            if !new_name.eq_ignore_ascii_case(&rel) {
                check_length(&new_name)?;
                let target = self.root.join(&new_name);
                fs_ops::rename_with_retry(&dir, &target)?;
                dir = target;
                rel = new_name;
            }
        }
        let file = CollectionFile {
            schema_version: COLLECTION_SCHEMA_VERSION,
            name,
            description: clean_description(description),
        };
        fs_ops::write_atomic(&dir.join(COLLECTION_FILE), &to_file_bytes(&file))?;
        self.node_of_container(&rel)
    }

    pub fn collection_delete(&self, rel: &str) -> AppResult<()> {
        fs_ops::remove_dir_all(&self.collection_dir(rel)?)
    }

    // ----- pastas -----

    pub fn folder_create(&self, parent: &str, name: &str) -> AppResult<TreeNode> {
        let parent_dir = self.container_dir(parent)?;
        let name = clean_name(name)?;
        let dir_name = unique_name(&existing_names(&parent_dir), &slugify(&name), "", &[]);
        let rel = join_rel(parent, &dir_name);
        check_length(&rel)?;
        fs_ops::create_dir_with_gitkeep(&parent_dir.join(&dir_name))?;
        self.node_of_container(&rel)
    }

    pub fn folder_rename(&self, rel: &str, name: &str) -> AppResult<TreeNode> {
        let dir = self.folder_dir(rel)?;
        let name = clean_name(name)?;
        let current = file_name_rel(rel).to_string();
        let slug = slugify(&name);
        if slug.eq_ignore_ascii_case(&current) {
            return self.node_of_container(rel);
        }
        let parent = parent_rel(rel);
        let parent_dir = dir.parent().expect("pasta sempre tem pai").to_path_buf();
        let new_name = unique_name(
            &taken_except(&parent_dir, std::slice::from_ref(&current)),
            &slug,
            "",
            &[],
        );
        if new_name.eq_ignore_ascii_case(&current) {
            return self.node_of_container(rel);
        }
        let new_rel = join_rel(parent, &new_name);
        check_length(&new_rel)?;
        fs_ops::rename_with_retry(&dir, &parent_dir.join(&new_name))?;
        self.node_of_container(&new_rel)
    }

    pub fn folder_delete(&self, rel: &str) -> AppResult<()> {
        fs_ops::remove_dir_all(&self.folder_dir(rel)?)
    }

    // ----- requests -----

    pub fn request_get(&self, rel: &str) -> AppResult<RequestFile> {
        let (dir, stem) = self.request_location(rel)?;
        let path = dir.join(format!("{stem}.json"));
        let text = std::fs::read_to_string(&path).map_err(|e| AppError::io(&e, &path))?;
        let disk = load::<DiskRequest>(&text, REQUEST_SCHEMA_VERSION, &[])?.value;
        let kind = match disk.body {
            DiskBody::Json => Some(BodyFileKind::Json),
            DiskBody::Text => Some(BodyFileKind::Text),
            _ => None,
        };
        let content = match kind {
            Some(k) => {
                let body_path = dir.join(format!("{stem}{}", k.suffix()));
                match std::fs::read_to_string(&body_path) {
                    Ok(c) => Some(c),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => return Err(AppError::io(&e, &body_path)),
                }
            }
            None => None,
        };
        Ok(RequestFile::join(disk, content))
    }

    /// Cria o request. Um token Bearer literal faz o save falhar com
    /// `BEARER_NEEDS_ENVIRONMENT`: o token nunca vai para o arquivo.
    pub fn request_create(&self, parent: &str, request: &RequestFile) -> AppResult<String> {
        Ok(self.request_create_in_env(parent, request, None)?.path)
    }

    /// Cria o request, guardando um token Bearer literal no segredo `bearer_token` do `env`.
    pub fn request_create_in_env(
        &self,
        parent: &str,
        request: &RequestFile,
        env: Option<&str>,
    ) -> AppResult<SavedRequest> {
        let dir = self.container_dir(parent)?;
        let mut request = Self::validated(request)?;
        let stem = unique_name(
            &existing_names(&dir),
            &slugify(&request.name),
            ".json",
            &BODY_SUFFIXES,
        );
        let rel = join_rel(parent, &format!("{stem}.json"));
        check_length(&rel)?;
        let bearer = bearer::extract(&self.root, &mut request, env)?;
        write_request(&dir, &stem, &request)?;
        Ok(SavedRequest {
            path: rel,
            request,
            bearer,
        })
    }

    /// Salva o request. Se o nome mudou de slug, renomeia os arquivos e devolve o novo caminho.
    /// Um request removido do disco é recriado.
    pub fn request_save(&self, rel: &str, request: &RequestFile) -> AppResult<String> {
        Ok(self.request_save_in_env(rel, request, None)?.path)
    }

    /// Como `request_save`, guardando um token Bearer literal no segredo do `env`.
    pub fn request_save_in_env(
        &self,
        rel: &str,
        request: &RequestFile,
        env: Option<&str>,
    ) -> AppResult<SavedRequest> {
        let (dir, stem) = self.request_location(rel)?;
        let mut request = Self::validated(request)?;
        let current = dir.join(format!("{stem}.json"));
        if let Ok(text) = std::fs::read_to_string(&current) {
            if let Err(e @ AppError::UnsupportedSchemaVersion(_)) =
                load::<DiskRequest>(&text, REQUEST_SCHEMA_VERSION, &[])
            {
                return Err(e);
            }
        }
        let parent = parent_rel(rel);
        let slug = slugify(&request.name);
        let mut new_stem = stem.clone();
        if !slug.eq_ignore_ascii_case(&stem) {
            let own = own_file_names(&stem);
            new_stem = unique_name(&taken_except(&dir, &own), &slug, ".json", &BODY_SUFFIXES);
        }
        let new_rel = join_rel(parent, &format!("{new_stem}.json"));
        if !new_stem.eq_ignore_ascii_case(&stem) {
            check_length(&new_rel)?;
        }
        // Antes de mexer nos arquivos do request: se o segredo falhar, nada muda.
        let bearer = bearer::extract(&self.root, &mut request, env)?;
        if !new_stem.eq_ignore_ascii_case(&stem) {
            for suffix in std::iter::once(".json").chain(BODY_SUFFIXES) {
                let from = dir.join(format!("{stem}{suffix}"));
                if from.exists() {
                    fs_ops::rename_with_retry(&from, &dir.join(format!("{new_stem}{suffix}")))?;
                }
            }
        }
        write_request(&dir, &new_stem, &request)?;
        Ok(SavedRequest {
            path: new_rel,
            request,
            bearer,
        })
    }

    /// Converte requests com token Bearer literal (botão "Converter agora" do push).
    /// Todos precisam ter o mesmo token, porque o segredo `bearer_token` guarda um só.
    pub fn convert_bearer(
        &self,
        rels: &[String],
        env: Option<&str>,
    ) -> AppResult<Option<BearerOutcome>> {
        let mut loaded = Vec::new();
        let mut tokens: Vec<String> = Vec::new();
        for rel in rels {
            let request = self.request_get(rel)?;
            if let Some(token) = bearer::single_token(&request)
                .map_err(|e| AppError::InvalidRequest(format!("{rel}: {e}")))?
            {
                if !tokens.contains(&token) {
                    tokens.push(token);
                }
                loaded.push((rel, request));
            }
        }
        if tokens.len() > 1 {
            let names: Vec<&str> = loaded.iter().map(|(r, _)| r.as_str()).collect();
            return Err(AppError::InvalidRequest(format!(
                "Estes requests têm tokens Bearer diferentes: {}. O segredo {} guarda um só; abra e salve cada um no ambiente certo.",
                names.join(", "),
                bearer::BEARER_SECRET_NAME
            )));
        }
        let mut first: Option<BearerOutcome> = None;
        for (rel, request) in loaded {
            let saved = self.request_save_in_env(rel, &request, env)?;
            if first.is_none() {
                first = saved.bearer;
            }
        }
        Ok(first)
    }

    pub fn request_delete(&self, rel: &str) -> AppResult<()> {
        let (dir, stem) = self.request_location(rel)?;
        let main = dir.join(format!("{stem}.json"));
        if !main.exists() {
            return Err(AppError::not_found(rel));
        }
        for suffix in BODY_SUFFIXES {
            fs_ops::remove_file_if_exists(&dir.join(format!("{stem}{suffix}")))?;
        }
        fs_ops::remove_file_if_exists(&main)
    }

    /// Duplica com o sufixo " (cópia)" no nome, na mesma pasta (RF-18).
    pub fn request_duplicate(&self, rel: &str, env: Option<&str>) -> AppResult<String> {
        let mut request = self.request_get(rel)?;
        request.name = format!("{} (cópia)", request.name);
        Ok(self
            .request_create_in_env(parent_rel(rel), &request, env)?
            .path)
    }

    /// Move o request (o `.json` e os arquivos de body) para outra pasta do workspace,
    /// sem ler nem regravar o conteúdo. Se o nome do arquivo já existir no destino,
    /// ganha o sufixo `-2`, `-3`…
    pub fn request_move(&self, rel: &str, target_parent: &str) -> AppResult<MovedRequest> {
        let (dir, stem) = self.request_location(rel)?;
        let main = dir.join(format!("{stem}.json"));
        if !main.is_file() {
            return Err(AppError::not_found(rel));
        }
        let target_dir = self.container_dir(target_parent)?;
        if same_dir(&dir, &target_dir) {
            return Err(AppError::InvalidRequest(
                "O request já está nessa pasta.".into(),
            ));
        }
        let new_stem = unique_name(&existing_names(&target_dir), &stem, ".json", &BODY_SUFFIXES);
        let new_rel = join_rel(target_parent, &format!("{new_stem}.json"));
        check_length(&new_rel)?;
        // O `.json` vai por último: é ele que faz o request aparecer na árvore.
        let moves: Vec<(PathBuf, PathBuf)> = BODY_SUFFIXES
            .iter()
            .chain(std::iter::once(&".json"))
            .map(|suffix| {
                (
                    dir.join(format!("{stem}{suffix}")),
                    target_dir.join(format!("{new_stem}{suffix}")),
                )
            })
            .filter(|(from, _)| from.is_file())
            .collect();
        move_all(&moves, fs_ops::rename_with_retry)?;
        Ok(MovedRequest {
            path: new_rel,
            renamed: new_stem != stem,
        })
    }

    fn validated(request: &RequestFile) -> AppResult<RequestFile> {
        let mut request = request.clone();
        request.name = clean_name(&request.name)?;
        request.schema_version = REQUEST_SCHEMA_VERSION;
        Ok(request)
    }
}

/// `true` se os dois caminhos apontam para o mesmo diretório (no Windows, sem diferenciar maiúsculas).
fn same_dir(a: &Path, b: &Path) -> bool {
    match (dunce::canonicalize(a), dunce::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Faz os renames em ordem. Se um falhar, desfaz os anteriores (em ordem inversa)
/// e devolve o erro: os arquivos nunca ficam divididos entre origem e destino.
fn move_all(
    moves: &[(PathBuf, PathBuf)],
    mut rename: impl FnMut(&Path, &Path) -> AppResult<()>,
) -> AppResult<()> {
    for (i, (from, to)) in moves.iter().enumerate() {
        if let Err(err) = rename(from, to) {
            for (done_from, done_to) in moves[..i].iter().rev() {
                let _ = rename(done_to, done_from);
            }
            return Err(err);
        }
    }
    Ok(())
}

fn own_file_names(stem: &str) -> Vec<String> {
    std::iter::once(".json")
        .chain(BODY_SUFFIXES)
        .map(|s| format!("{stem}{s}"))
        .collect()
}

/// Grava o body (se houver) e depois o request; remove arquivos de body que não valem mais.
fn write_request(dir: &Path, stem: &str, request: &RequestFile) -> AppResult<()> {
    let (disk, body) = request.split();
    for kind in BodyFileKind::ALL {
        let path = dir.join(format!("{stem}{}", kind.suffix()));
        match &body {
            Some((k, content)) if *k == kind => {
                fs_ops::write_atomic(&path, content.as_bytes())?;
            }
            _ => fs_ops::remove_file_if_exists(&path)?,
        }
    }
    fs_ops::write_atomic(&dir.join(format!("{stem}.json")), &to_file_bytes(&disk))?;
    Ok(())
}

fn canonical_dir(path: &Path) -> AppResult<PathBuf> {
    let root = dunce::canonicalize(path)
        .map_err(|_| AppError::NotFound(format!("Pasta não encontrada: {}", path.display())))?;
    if !root.is_dir() {
        return Err(AppError::NotFound(format!(
            "Não é uma pasta: {}",
            path.display()
        )));
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs() -> Vec<(PathBuf, PathBuf)> {
        vec![
            (
                PathBuf::from("a/x.body.json"),
                PathBuf::from("b/x.body.json"),
            ),
            (PathBuf::from("a/x.json"), PathBuf::from("b/x.json")),
        ]
    }

    #[test]
    fn move_all_undoes_previous_renames_when_one_fails() {
        let mut calls: Vec<(PathBuf, PathBuf)> = Vec::new();
        let err = move_all(&pairs(), |from, to| {
            calls.push((from.to_path_buf(), to.to_path_buf()));
            if from.ends_with("x.json") {
                Err(AppError::IoError("em uso".into()))
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(err.code(), "IO_ERROR");
        assert_eq!(
            calls,
            vec![
                (
                    PathBuf::from("a/x.body.json"),
                    PathBuf::from("b/x.body.json")
                ),
                (PathBuf::from("a/x.json"), PathBuf::from("b/x.json")),
                (
                    PathBuf::from("b/x.body.json"),
                    PathBuf::from("a/x.body.json")
                ),
            ],
            "o body volta para a origem"
        );
    }

    #[test]
    fn move_all_renames_in_order() {
        let mut calls = Vec::new();
        move_all(&pairs(), |from, _| {
            calls.push(from.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(
            calls,
            vec![PathBuf::from("a/x.body.json"), PathBuf::from("a/x.json")]
        );
    }
}
