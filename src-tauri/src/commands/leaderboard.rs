//! 排行榜相关命令

use crate::error::AppError;
use crate::services::leaderboard::{
    GithubDeviceLoginStart, GithubDeviceLoginStatus, LeaderboardEntriesResult, LeaderboardProfile,
    LeaderboardRange,
};
use crate::store::AppState;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_leaderboard_profile(
    state: State<'_, AppState>,
) -> Result<Option<LeaderboardProfile>, AppError> {
    state.db.get_leaderboard_profile()
}

#[tauri::command]
pub async fn start_github_leaderboard_login(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<GithubDeviceLoginStart, AppError> {
    let login = crate::services::leaderboard::start_github_device_login(&state.db).await?;

    app.opener()
        .open_url(&login.verification_uri, None::<String>)
        .map_err(|e| AppError::Message(format!("打开 GitHub 登录失败: {e}")))?;

    Ok(login)
}

#[tauri::command]
pub async fn poll_github_leaderboard_login(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<GithubDeviceLoginStatus, AppError> {
    crate::services::leaderboard::poll_github_device_login(&state.db, &session_id).await
}

#[tauri::command]
pub fn sign_out_leaderboard(state: State<'_, AppState>) -> Result<(), AppError> {
    state.db.sign_out_leaderboard()
}

#[tauri::command]
pub async fn set_leaderboard_opt_in(
    state: State<'_, AppState>,
    opted_in: bool,
) -> Result<LeaderboardProfile, AppError> {
    crate::services::leaderboard::set_remote_leaderboard_opt_in(&state.db, opted_in).await
}

#[tauri::command]
pub async fn get_leaderboard_entries(
    state: State<'_, AppState>,
    range: String,
) -> Result<LeaderboardEntriesResult, AppError> {
    let range = LeaderboardRange::parse(&range)?;
    crate::services::leaderboard::get_remote_leaderboard_entries(&state.db, range).await
}
