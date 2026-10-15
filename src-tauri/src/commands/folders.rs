use crate::state::AppState;
use anansi_core::model::TreeNode;
use anansi_core::AppResult;
use tauri::State;

#[tauri::command]
pub async fn folder_create(
    state: State<'_, AppState>,
    parent_path: String,
    name: String,
) -> AppResult<TreeNode> {
    state.workspace()?.folder_create(&parent_path, &name)
}

#[tauri::command]
pub async fn folder_rename(
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> AppResult<TreeNode> {
    state.workspace()?.folder_rename(&path, &name)
}

#[tauri::command]
pub async fn folder_delete(state: State<'_, AppState>, path: String) -> AppResult<()> {
    state.workspace()?.folder_delete(&path)
}
