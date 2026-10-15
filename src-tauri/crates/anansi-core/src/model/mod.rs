//! Modelos serializáveis (Seção 4 da spec).

pub mod request;
pub mod schema;

use serde::{Deserialize, Serialize};

pub use request::*;

pub const WORKSPACE_SCHEMA_VERSION: u32 = 1;
pub const COLLECTION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFile {
    pub schema_version: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionFile {
    pub schema_version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Dados do workspace aberto, enviados ao frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub path: String,
    pub name: String,
    pub environments: Vec<String>,
    pub gitignore_ok: bool,
}

/// Nó da árvore. O `path` é relativo ao workspace, com `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TreeNode {
    Collection {
        path: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        children: Vec<TreeNode>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Folder {
        path: String,
        name: String,
        children: Vec<TreeNode>,
    },
    Request {
        path: String,
        name: String,
        method: String,
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(rename = "readOnly", skip_serializing_if = "std::ops::Not::not")]
        read_only: bool,
    },
}

impl TreeNode {
    pub fn path(&self) -> &str {
        match self {
            TreeNode::Collection { path, .. }
            | TreeNode::Folder { path, .. }
            | TreeNode::Request { path, .. } => path,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            TreeNode::Collection { name, .. }
            | TreeNode::Folder { name, .. }
            | TreeNode::Request { name, .. } => name,
        }
    }

    pub fn children(&self) -> &[TreeNode] {
        match self {
            TreeNode::Collection { children, .. } | TreeNode::Folder { children, .. } => children,
            TreeNode::Request { .. } => &[],
        }
    }

    /// Quantidade de requests neste nó e abaixo dele.
    pub fn request_count(&self) -> usize {
        match self {
            TreeNode::Request { .. } => 1,
            _ => self.children().iter().map(TreeNode::request_count).sum(),
        }
    }
}

/// Serialização estável dos arquivos (RNF-04): 2 espaços, LF e quebra final.
pub fn to_file_bytes<T: Serialize>(value: &T) -> Vec<u8> {
    let mut text = serde_json::to_string_pretty(value).expect("modelos sempre serializam");
    text.push('\n');
    text.into_bytes()
}
