use crate::state::AppState;
use anansi_core::model::TreeNode;
use anansi_core::AppResult;
use tauri::State;

#[tauri::command]
pub async fn collection_create(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> AppResult<TreeNode> {
    state
        .workspace()?
        .collection_create(&name, description.as_deref())
}

#[tauri::command]
pub async fn collection_update(
    state: State<'_, AppState>,
    path: String,
    name: String,
    description: Option<String>,
) -> AppResult<TreeNode> {
    state
        .workspace()?
        .collection_update(&path, &name, description.as_deref())
}

#[tauri::command]
pub async fn collection_delete(state: State<'_, AppState>, path: String) -> AppResult<()> {
    state.workspace()?.collection_delete(&path)
}
