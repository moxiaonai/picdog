use crate::error::CommandResult;
use crate::models::AppSettings;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CommandResult<AppSettings> {
    state.get_settings()
}

#[tauri::command]
pub async fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> CommandResult<()> {
    state.save_settings(&settings)
}
