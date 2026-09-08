use tauri::State;
use crate::error::AppError;
use crate::inference::InferenceEngineManager;

#[tauri::command]
pub async fn unload_inference_model(
    state: State<'_, InferenceEngineManager>,
) -> Result<(), AppError> {
    state.unload_model().await;
    Ok(())
}

#[tauri::command]
pub async fn is_inference_model_loaded(
    model_id: String,
    state: State<'_, InferenceEngineManager>,
) -> Result<bool, AppError> {
    Ok(state.is_model_loaded(&model_id).await)
}
