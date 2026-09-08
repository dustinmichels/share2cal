use crate::error::AppError;
use crate::model;

#[tauri::command]
pub fn get_model_manifest() -> Result<model::ModelManifest, AppError> {
    model::get_manifest()
}

#[tauri::command]
pub fn get_model_statuses(app: tauri::AppHandle) -> Result<Vec<model::ModelStatus>, AppError> {
    model::get_all_model_statuses(&app)
}

#[tauri::command]
pub fn get_model_status(app: tauri::AppHandle, model_id: String) -> Result<model::ModelStatus, AppError> {
    model::get_single_model_status(&app, &model_id)
}

#[tauri::command]
pub async fn download_model(app: tauri::AppHandle, model_id: String) -> Result<(), AppError> {
    model::start_model_download(app, model_id).await
}

#[tauri::command]
pub fn cancel_model_download(app: tauri::AppHandle, model_id: String) -> Result<(), AppError> {
    model::cancel_download(&app, &model_id)
}

#[tauri::command]
pub async fn delete_model(app: tauri::AppHandle, model_id: String) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(move || model::delete_model_file(&app, &model_id))
        .await
        .map_err(|e| AppError::Io(format!("Delete task failed: {}", e)))?
}

#[tauri::command]
pub async fn verify_model_hash(app: tauri::AppHandle, model_id: String) -> Result<bool, AppError> {
    tauri::async_runtime::spawn_blocking(move || model::verify_model(&app, &model_id))
        .await
        .map_err(|e| AppError::Io(format!("Verification task failed: {}", e)))?
}

#[tauri::command]
pub fn get_models_storage_info(app: tauri::AppHandle) -> Result<model::ModelsStorageInfo, AppError> {
    model::get_models_storage_info(&app)
}

#[tauri::command]
pub fn open_models_directory(app: tauri::AppHandle) -> Result<(), AppError> {
    model::open_models_directory(&app)
}
