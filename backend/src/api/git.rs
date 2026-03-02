use crate::models::AppState;
use crate::system;
use axum::{extract::Query, http::StatusCode, response::IntoResponse, routing, Json, Router};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct DiffQuery {
    pub file: Option<String>,
}

#[derive(Deserialize)]
pub struct AddRequest {
    pub files: Vec<String>,
}

#[derive(Deserialize)]
pub struct RevertRequest {
    pub commit_hash: Option<String>,
    pub file: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", routing::get(get_git_status))
        .route("/init", routing::post(init_git_repo))
        .route("/add", routing::post(add_files_to_git))
        .route("/commit", routing::post(commit_changes))
        .route("/history", routing::get(get_git_history))
        .route("/diff", routing::get(get_git_diff))
        .route("/revert", routing::post(revert_changes))
        .route("/remote/configure", routing::post(configure_remote_repo))
        .route("/remote/pull", routing::post(pull_from_remote_repo))
        .route("/remote/push", routing::post(push_to_remote_repo))
        .route("/stacks/available", routing::get(get_available_stacks))
        .route("/stacks/import", routing::post(import_selected_stacks))
        .route("/stacks/activate", routing::post(activate_selected_stack))
        .route(
            "/stacks/deactivate",
            routing::post(deactivate_selected_stack),
        )
        .route("/stacks/active", routing::get(get_active_stacks))
        .route("/repo/diagnose", routing::get(diagnose_repository_content))
        .route("/repo/stacks", routing::get(get_all_repo_stacks))
}

/// GET /git/status - Obtiene el estado del repositorio git
async fn get_git_status() -> impl IntoResponse {
    match system::get_git_status().await {
        Ok(status) => Json(status),
        Err(e) => {
            eprintln!("Error getting git status: {}", e);
            Json(system::GitStatus {
                is_repo: false,
                branch: None,
                staged: vec![],
                modified: vec![],
                untracked: vec![],
                commits_ahead: 0,
                commits_behind: 0,
            })
        }
    }
}

/// POST /git/init - Inicializa un repositorio git en el directorio de quadlets
async fn init_git_repo() -> impl IntoResponse {
    match system::init_repo().await {
        Ok(_) => (StatusCode::OK, "Git repository initialized successfully"),
        Err(e) => {
            eprintln!("Error initializing git repo: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to initialize git repository",
            )
        }
    }
}

/// POST /git/add - Agrega archivos al staging area
async fn add_files_to_git(Json(payload): Json<AddRequest>) -> impl IntoResponse {
    match system::add_files(payload.files).await {
        Ok(_) => (StatusCode::OK, "Files added to staging area"),
        Err(e) => {
            eprintln!("Error adding files to git: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to add files")
        }
    }
}

/// POST /git/commit - Hace commit de los cambios
async fn commit_changes(Json(payload): Json<system::CommitRequest>) -> impl IntoResponse {
    match system::commit(&payload.message, payload.files).await {
        Ok(commit_hash) => Json(serde_json::json!({
            "commit_hash": commit_hash,
            "message": "Changes committed successfully"
        })),
        Err(e) => {
            eprintln!("Error committing changes: {}", e);
            Json(serde_json::json!({
                "error": "Failed to commit changes",
                "details": e.to_string()
            }))
        }
    }
}

/// GET /git/history - Obtiene el historial de commits
async fn get_git_history(Query(params): Query<HistoryQuery>) -> impl IntoResponse {
    match system::get_history(params.limit).await {
        Ok(history) => Json(history),
        Err(e) => {
            eprintln!("Error getting git history: {}", e);
            Json(vec![])
        }
    }
}

/// GET /git/diff - Obtiene las diferencias de archivos
async fn get_git_diff(Query(params): Query<DiffQuery>) -> impl IntoResponse {
    match system::get_diff(params.file.as_deref()).await {
        Ok(diff) => (StatusCode::OK, diff),
        Err(e) => {
            eprintln!("Error getting git diff: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get diff".to_string(),
            )
        }
    }
}

/// POST /git/revert - Revierte cambios
async fn revert_changes(Json(payload): Json<RevertRequest>) -> impl IntoResponse {
    if let Some(file) = payload.file {
        // Revertir archivo específico
        match system::revert_file(&file).await {
            Ok(_) => (StatusCode::OK, "File reverted successfully"),
            Err(e) => {
                eprintln!("Error reverting file: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to revert file")
            }
        }
    } else if let Some(commit_hash) = payload.commit_hash {
        // Revertir a commit específico
        match system::revert_to_commit(&commit_hash).await {
            Ok(_) => (StatusCode::OK, "Reverted to commit successfully"),
            Err(e) => {
                eprintln!("Error reverting to commit: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to revert to commit",
                )
            }
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            "Must specify either file or commit_hash",
        )
    }
}

/// POST /git/remote/configure - Configura el repositorio remoto
async fn configure_remote_repo(
    Json(payload): Json<system::RemoteConfigRequest>,
) -> impl IntoResponse {
    let branch = payload.branch.as_deref();

    match system::configure_remote(&payload.url, branch).await {
        Ok(_) => (StatusCode::OK, "Remote repository configured successfully"),
        Err(e) => {
            eprintln!("Error configuring remote: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to configure remote repository",
            )
        }
    }
}

/// POST /git/remote/pull - Sincroniza cambios desde el repositorio remoto
async fn pull_from_remote_repo() -> impl IntoResponse {
    match system::pull_from_remote().await {
        Ok(output) => Json(serde_json::json!({
            "message": "Successfully pulled from remote",
            "output": output
        })),
        Err(e) => {
            eprintln!("Error pulling from remote: {}", e);
            Json(serde_json::json!({
                "error": "Failed to pull from remote",
                "details": e.to_string()
            }))
        }
    }
}

/// POST /git/remote/push - Envía cambios al repositorio remoto
async fn push_to_remote_repo() -> impl IntoResponse {
    match system::push_to_remote().await {
        Ok(output) => Json(serde_json::json!({
            "message": "Successfully pushed to remote",
            "output": output
        })),
        Err(e) => {
            eprintln!("Error pushing to remote: {}", e);
            Json(serde_json::json!({
                "error": "Failed to push to remote",
                "details": e.to_string()
            }))
        }
    }
}

/// GET /git/stacks/available - Lista los stacks disponibles
async fn get_available_stacks() -> impl IntoResponse {
    match system::list_available_stacks().await {
        Ok(stacks) => Json(stacks),
        Err(e) => {
            eprintln!("Error getting available stacks: {}", e);
            Json(vec![])
        }
    }
}

/// POST /git/stacks/import - Importa stacks seleccionados
async fn import_selected_stacks(
    Json(payload): Json<system::ImportStackRequest>,
) -> impl IntoResponse {
    match system::import_stacks(payload.stacks).await {
        Ok(imported) => Json(serde_json::json!({
            "message": "Stacks imported successfully",
            "imported_files": imported
        })),
        Err(e) => {
            eprintln!("Error importing stacks: {}", e);
            Json(serde_json::json!({
                "error": "Failed to import stacks",
                "details": e.to_string()
            }))
        }
    }
}

/// POST /git/stacks/activate - Activa un stack creando enlaces simbólicos
async fn activate_selected_stack(
    Json(payload): Json<system::ActivateStackRequest>,
) -> impl IntoResponse {
    match system::activate_stack(&payload.stack_name).await {
        Ok(activated_files) => Json(serde_json::json!({
            "message": format!("Stack '{}' activated successfully", payload.stack_name),
            "activated_files": activated_files
        })),
        Err(e) => {
            eprintln!("Error activating stack '{}': {}", payload.stack_name, e);
            Json(serde_json::json!({
                "error": "Failed to activate stack",
                "details": e.to_string()
            }))
        }
    }
}

/// POST /git/stacks/deactivate - Desactiva un stack eliminando enlaces simbólicos
async fn deactivate_selected_stack(
    Json(payload): Json<system::DeactivateStackRequest>,
) -> impl IntoResponse {
    match system::deactivate_stack(&payload.stack_name).await {
        Ok(deactivated_files) => Json(serde_json::json!({
            "message": format!("Stack '{}' deactivated successfully", payload.stack_name),
            "deactivated_files": deactivated_files
        })),
        Err(e) => {
            eprintln!("Error deactivating stack '{}': {}", payload.stack_name, e);
            Json(serde_json::json!({
                "error": "Failed to deactivate stack",
                "details": e.to_string()
            }))
        }
    }
}

/// GET /git/stacks/active - Lista los stacks actualmente activos
async fn get_active_stacks() -> impl IntoResponse {
    match system::list_active_stacks().await {
        Ok(stacks) => Json(stacks),
        Err(e) => {
            eprintln!("Error getting active stacks: {}", e);
            Json(vec![])
        }
    }
}

/// GET /git/repo/diagnose - Diagnóstica el contenido del repositorio
async fn diagnose_repository_content() -> impl IntoResponse {
    match system::diagnose_repo_content().await {
        Ok(content) => Json(content),
        Err(e) => {
            eprintln!("Error diagnosing repository content: {}", e);
            Json(system::RepoContent {
                path: "Error".to_string(),
                entries: vec![],
                quadlet_files: vec![],
                directories: vec![],
            })
        }
    }
}

/// GET /git/repo/stacks - Lista todos los stacks encontrados en el repositorio
async fn get_all_repo_stacks() -> impl IntoResponse {
    match system::list_all_repo_stacks().await {
        Ok(stacks) => Json(stacks),
        Err(e) => {
            eprintln!("Error getting all repo stacks: {}", e);
            Json(vec![])
        }
    }
}
