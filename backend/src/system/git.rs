use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;
use ts_rs::TS;

use crate::models::{get_git_repo_dir, get_quadlet_dir};
use crate::system::systemd;

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/GitStatus.ts")]
pub struct GitStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub staged: Vec<String>,
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
    pub commits_ahead: i32,
    pub commits_behind: i32,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/GitCommit.ts")]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub files: Vec<String>,
}

#[derive(Deserialize)]
pub struct CommitRequest {
    pub message: String,
    pub files: Option<Vec<String>>, // Si es None, hace commit de todos los archivos staged
}

/// Inicializa un repositorio git en el directorio de repositorios si no existe
pub async fn init_repo() -> Result<()> {
    let git_repo_dir = get_git_repo_dir();

    // Crear el directorio si no existe
    if !git_repo_dir.exists() {
        tokio::fs::create_dir_all(&git_repo_dir).await?;
    }

    if !is_git_repo().await? {
        let output = TokioCommand::new("git")
            .args(&["init"])
            .current_dir(&git_repo_dir)
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Error initializing git repo: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    Ok(())
}

/// Verifica si el directorio de repositorios git es un repositorio git
pub async fn is_git_repo() -> Result<bool> {
    let git_repo_dir = get_git_repo_dir();

    // Primero verificar si el directorio existe
    if !git_repo_dir.exists() {
        return Ok(false);
    }

    let output = TokioCommand::new("git")
        .args(&["rev-parse", "--git-dir"])
        .current_dir(&git_repo_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    Ok(output.success())
}

/// Obtiene el status actual del repositorio git
pub async fn get_status() -> Result<GitStatus> {
    let git_repo_dir = get_git_repo_dir();

    // Verificar si el directorio existe y es un repositorio git
    if !git_repo_dir.exists() || !is_git_repo().await? {
        return Ok(GitStatus {
            is_repo: false,
            branch: None,
            staged: vec![],
            modified: vec![],
            untracked: vec![],
            commits_ahead: 0,
            commits_behind: 0,
        });
    }

    // Obtener branch actual
    let branch_output = TokioCommand::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    let branch = if branch_output.status.success() {
        Some(
            String::from_utf8_lossy(&branch_output.stdout)
                .trim()
                .to_string(),
        )
    } else {
        None
    };

    // Obtener status de archivos
    let status_output = TokioCommand::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    let mut staged = vec![];
    let mut modified = vec![];
    let mut untracked = vec![];

    if status_output.status.success() {
        let status_lines = String::from_utf8_lossy(&status_output.stdout);
        for line in status_lines.lines() {
            if line.len() >= 3 {
                let status_code = &line[0..2];
                let filename = line[3..].to_string();

                match status_code {
                    "A " | "M " | "D " => staged.push(filename),
                    " M" | " D" => modified.push(filename),
                    "??" => untracked.push(filename),
                    "AM" | "MM" => {
                        staged.push(filename.clone());
                        modified.push(filename);
                    }
                    _ => {}
                }
            }
        }
    }

    // Obtener commits ahead/behind (solo si hay remote)
    let mut commits_ahead = 0;
    let mut commits_behind = 0;

    if let Ok(remote_output) = TokioCommand::new("git")
        .args(&["rev-list", "--count", "--left-right", "HEAD...@{upstream}"])
        .current_dir(&git_repo_dir)
        .output()
        .await
    {
        if remote_output.status.success() {
            let count_str = String::from_utf8_lossy(&remote_output.stdout);
            if let Some(parts) = count_str.trim().split_once('\t') {
                commits_ahead = parts.0.parse().unwrap_or(0);
                commits_behind = parts.1.parse().unwrap_or(0);
            }
        }
    }

    Ok(GitStatus {
        is_repo: true,
        branch,
        staged,
        modified,
        untracked,
        commits_ahead,
        commits_behind,
    })
}

/// Agrega archivos al staging area
pub async fn add_files(files: Vec<String>) -> Result<()> {
    let git_repo_dir = get_git_repo_dir();

    let output = TokioCommand::new("git")
        .args(&["add"])
        .args(&files)
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Error adding files to git: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Hace commit de los archivos staged
pub async fn commit(message: &str, files: Option<Vec<String>>) -> Result<String> {
    let git_repo_dir = get_git_repo_dir();

    // Si se especifican archivos, agregarlos primero
    if let Some(files) = files {
        add_files(files).await?;
    }

    let output = TokioCommand::new("git")
        .args(&["commit", "-m", message])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Error committing changes: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Obtener el hash del commit
    let hash_output = TokioCommand::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    let commit_hash = String::from_utf8_lossy(&hash_output.stdout)
        .trim()
        .to_string();
    Ok(commit_hash)
}

/// Obtiene el historial de commits
pub async fn get_history(limit: Option<u32>) -> Result<Vec<GitCommit>> {
    let git_repo_dir = get_git_repo_dir();
    let limit_str = limit.unwrap_or(20).to_string();

    let output = TokioCommand::new("git")
        .args(&[
            "log",
            &format!("-{}", limit_str),
            "--pretty=format:%H|%an|%ad|%s",
            "--date=iso",
            "--name-only",
        ])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Error getting git history: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let log_output = String::from_utf8_lossy(&output.stdout);
    let mut commits = vec![];
    let mut current_commit: Option<GitCommit> = None;

    for line in log_output.lines() {
        if line.contains('|') && line.chars().next().map_or(false, |c| c.is_ascii_hexdigit()) {
            // Nueva entrada de commit
            if let Some(commit) = current_commit.take() {
                commits.push(commit);
            }

            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() == 4 {
                current_commit = Some(GitCommit {
                    hash: parts[0].to_string(),
                    author: parts[1].to_string(),
                    date: parts[2].to_string(),
                    message: parts[3].to_string(),
                    files: vec![],
                });
            }
        } else if !line.is_empty() {
            // Archivo modificado en el commit
            if let Some(ref mut commit) = current_commit {
                commit.files.push(line.to_string());
            }
        }
    }

    // Agregar el último commit
    if let Some(commit) = current_commit {
        commits.push(commit);
    }

    Ok(commits)
}

/// Revierte cambios a un archivo específico
pub async fn revert_file(filename: &str) -> Result<()> {
    let git_repo_dir = get_git_repo_dir();

    let output = TokioCommand::new("git")
        .args(&["checkout", "HEAD", "--", filename])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Error reverting file {}: {}",
            filename,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Revierte a un commit específico
pub async fn revert_to_commit(commit_hash: &str) -> Result<()> {
    let git_repo_dir = get_git_repo_dir();

    let output = TokioCommand::new("git")
        .args(&["reset", "--hard", commit_hash])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Error reverting to commit {}: {}",
            commit_hash,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Obtiene las diferencias de un archivo
pub async fn get_diff(filename: Option<&str>) -> Result<String> {
    let git_repo_dir = get_git_repo_dir();

    let mut args = vec!["diff"];
    if let Some(file) = filename {
        args.push(file);
    }

    let output = TokioCommand::new("git")
        .args(&args)
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Error getting diff: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/StackInfo.ts")]
pub struct StackInfo {
    pub name: String,
    pub path: String,
    pub files: Vec<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct RemoteConfigRequest {
    pub url: String,
    pub branch: Option<String>,
}

#[derive(Deserialize)]
pub struct ImportStackRequest {
    pub stacks: Vec<String>,
}

/// Configura el repositorio remoto
pub async fn configure_remote(url: &str, branch: Option<&str>) -> Result<()> {
    let git_repo_dir = get_git_repo_dir();

    // Primero verificar si es un repositorio git, si no, inicializarlo
    if !is_git_repo().await? {
        init_repo().await?;
    }

    // Verificar si ya existe un remote origin
    let remote_output = TokioCommand::new("git")
        .args(&["remote", "get-url", "origin"])
        .current_dir(&git_repo_dir)
        .output()
        .await;

    match remote_output {
        Ok(output) if output.status.success() => {
            // Si existe, actualizarlo
            let update_output = TokioCommand::new("git")
                .args(&["remote", "set-url", "origin", url])
                .current_dir(&git_repo_dir)
                .output()
                .await?;

            if !update_output.status.success() {
                return Err(anyhow::anyhow!(
                    "Error updating remote origin: {}",
                    String::from_utf8_lossy(&update_output.stderr)
                ));
            }
        }
        _ => {
            // No existe, crearlo
            let add_output = TokioCommand::new("git")
                .args(&["remote", "add", "origin", url])
                .current_dir(&git_repo_dir)
                .output()
                .await?;

            if !add_output.status.success() {
                return Err(anyhow::anyhow!(
                    "Error adding remote origin: {}",
                    String::from_utf8_lossy(&add_output.stderr)
                ));
            }
        }
    }

    // Si se especifica branch, cambiar a ella después de hacer el primer fetch
    if let Some(branch_name) = branch {
        // Hacer fetch primero para obtener las ramas remotas
        let fetch_output = TokioCommand::new("git")
            .args(&["fetch", "origin"])
            .current_dir(&git_repo_dir)
            .output()
            .await;

        // Intentar hacer checkout de la rama, creándola si no existe localmente
        let branch_output = TokioCommand::new("git")
            .args(&[
                "checkout",
                "-B",
                branch_name,
                &format!("origin/{}", branch_name),
            ])
            .current_dir(&git_repo_dir)
            .output()
            .await;

        // Si falla el checkout con tracking, crear una nueva rama
        if branch_output.is_err() || !branch_output.as_ref().unwrap().status.success() {
            let create_output = TokioCommand::new("git")
                .args(&["checkout", "-b", branch_name])
                .current_dir(&git_repo_dir)
                .output()
                .await?;

            if !create_output.status.success() {
                return Err(anyhow::anyhow!(
                    "Error creating branch {}: {}",
                    branch_name,
                    String::from_utf8_lossy(&create_output.stderr)
                ));
            }
        }
    }

    Ok(())
}

/// Sincroniza cambios desde el repositorio remoto
pub async fn pull_from_remote() -> Result<String> {
    let git_repo_dir = get_git_repo_dir();

    // Verificar que existe un repositorio git
    if !is_git_repo().await? {
        return Err(anyhow::anyhow!(
            "No git repository found. Please configure a remote repository first."
        ));
    }

    // Verificar que existe un remote origin
    let remote_check = TokioCommand::new("git")
        .args(&["remote", "get-url", "origin"])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !remote_check.status.success() {
        return Err(anyhow::anyhow!(
            "No remote 'origin' configured. Please configure a remote repository first."
        ));
    }

    // Primero hacer fetch
    let fetch_output = TokioCommand::new("git")
        .args(&["fetch", "origin"])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !fetch_output.status.success() {
        return Err(anyhow::anyhow!(
            "Error fetching from remote: {}",
            String::from_utf8_lossy(&fetch_output.stderr)
        ));
    }

    // Luego hacer pull de la rama actual
    let pull_output = TokioCommand::new("git")
        .args(&["pull", "origin", "HEAD"])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !pull_output.status.success() {
        return Err(anyhow::anyhow!(
            "Error pulling from remote: {}",
            String::from_utf8_lossy(&pull_output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&pull_output.stdout).to_string())
}

/// Envía cambios al repositorio remoto
pub async fn push_to_remote() -> Result<String> {
    let git_repo_dir = get_git_repo_dir();

    let push_output = TokioCommand::new("git")
        .args(&["push", "origin", "HEAD"])
        .current_dir(&git_repo_dir)
        .output()
        .await?;

    if !push_output.status.success() {
        return Err(anyhow::anyhow!(
            "Error pushing to remote: {}",
            String::from_utf8_lossy(&push_output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&push_output.stdout).to_string())
}

/// Lista los stacks disponibles en .config/containers/available
pub async fn list_available_stacks() -> Result<Vec<StackInfo>> {
    let git_repo_dir = get_git_repo_dir();
    let available_path = git_repo_dir.join(".config/containers/available");

    if !available_path.exists() {
        return Ok(vec![]);
    }

    let mut stacks = vec![];
    let mut entries = tokio::fs::read_dir(&available_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let stack_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Buscar archivos quadlet en el directorio
            let mut files = vec![];
            let mut stack_entries = tokio::fs::read_dir(&path).await?;

            while let Some(stack_entry) = stack_entries.next_entry().await? {
                let file_path = stack_entry.path();
                if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
                    match extension {
                        "container" | "network" | "volume" | "kube" | "pod" | "image" => {
                            if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                                files.push(filename.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Buscar descripción en README.md o description.txt
            let mut description = None;
            for desc_file in ["README.md", "description.txt", "info.txt"] {
                let desc_path = path.join(desc_file);
                if desc_path.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(&desc_path).await {
                        description = Some(content.lines().next().unwrap_or("").to_string());
                        break;
                    }
                }
            }

            stacks.push(StackInfo {
                name: stack_name,
                path: path.to_string_lossy().to_string(),
                files,
                description,
            });
        }
    }

    Ok(stacks)
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/RepoContent.ts")]
pub struct RepoContent {
    pub path: String,
    pub entries: Vec<String>,
    pub quadlet_files: Vec<String>,
    pub directories: Vec<String>,
}

/// Función de diagnóstico para ver el contenido del repositorio
pub async fn diagnose_repo_content() -> Result<RepoContent> {
    let git_repo_dir = get_git_repo_dir();

    let mut entries = vec![];
    let mut quadlet_files = vec![];
    let mut directories = vec![];

    // Listar archivos en el directorio raíz del git repo
    if let Ok(mut dir_entries) = tokio::fs::read_dir(&git_repo_dir).await {
        while let Some(entry) = dir_entries.next_entry().await? {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            entries.push(name.clone());

            if path.is_dir() {
                directories.push(name);
            } else if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                match extension {
                    "container" | "network" | "volume" | "kube" | "pod" | "image" => {
                        quadlet_files.push(name);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(RepoContent {
        path: git_repo_dir.to_string_lossy().to_string(),
        entries,
        quadlet_files,
        directories,
    })
}

/// Lista stacks disponibles buscando en todo el repositorio (no solo en .config/containers/available)
pub async fn list_all_repo_stacks() -> Result<Vec<StackInfo>> {
    let git_repo_dir = get_git_repo_dir();
    let mut stacks = std::collections::HashMap::<String, StackInfo>::new();

    // Función recursiva para buscar archivos quadlet
    fn collect_quadlet_files(
        dir: &std::path::Path,
        base_path: &std::path::Path,
        stacks: &mut std::collections::HashMap<String, StackInfo>,
    ) -> Result<()> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir()
                        && !path
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or("")
                            .starts_with('.')
                    {
                        // Recursivamente buscar en subdirectorios
                        collect_quadlet_files(&path, base_path, stacks)?;
                    } else if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                        match extension {
                            "container" | "network" | "volume" | "kube" | "pod" | "image" => {
                                let filename = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("")
                                    .to_string();

                                // Usar el directorio padre como nombre del stack
                                let stack_name = if let Some(parent) = path.parent() {
                                    if parent == base_path {
                                        "root".to_string()
                                    } else {
                                        parent
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("unknown")
                                            .to_string()
                                    }
                                } else {
                                    "root".to_string()
                                };

                                let stack =
                                    stacks
                                        .entry(stack_name.clone())
                                        .or_insert_with(|| StackInfo {
                                            name: stack_name.clone(),
                                            path: path
                                                .parent()
                                                .unwrap_or(base_path)
                                                .to_string_lossy()
                                                .to_string(),
                                            files: vec![],
                                            description: None,
                                        });

                                stack.files.push(filename);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }

    collect_quadlet_files(&git_repo_dir, &git_repo_dir, &mut stacks)?;

    // Buscar descripciones para cada stack
    for stack in stacks.values_mut() {
        let stack_path = std::path::Path::new(&stack.path);
        for desc_file in ["README.md", "description.txt", "info.txt"] {
            let desc_path = stack_path.join(desc_file);
            if desc_path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&desc_path).await {
                    stack.description = Some(content.lines().next().unwrap_or("").to_string());
                    break;
                }
            }
        }
    }

    Ok(stacks.into_values().collect())
}

/// Importa stacks específicos desde .config/containers/available al directorio local
pub async fn import_stacks(stack_names: Vec<String>) -> Result<Vec<String>> {
    let git_repo_dir = get_git_repo_dir();
    let available_path = git_repo_dir.join(".config/containers/available");
    let mut imported = vec![];

    for stack_name in stack_names {
        let stack_path = available_path.join(&stack_name);
        if !stack_path.exists() {
            continue;
        }

        // Copiar archivos quadlet al directorio principal
        let mut entries = tokio::fs::read_dir(&stack_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_path = entry.path();
            if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
                match extension {
                    "container" | "network" | "volume" | "kube" | "pod" | "image" => {
                        if let Some(filename) = file_path.file_name() {
                            let dest_path = get_quadlet_dir().join(filename);

                            // Copiar archivo
                            tokio::fs::copy(&file_path, &dest_path).await?;

                            if let Some(name) = filename.to_str() {
                                imported.push(name.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(imported)
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/ActiveStack.ts")]
pub struct ActiveStack {
    pub name: String,
    pub files: Vec<String>,
    pub source_path: String,
}

#[derive(Deserialize)]
pub struct ActivateStackRequest {
    pub stack_name: String,
}

#[derive(Deserialize)]
pub struct DeactivateStackRequest {
    pub stack_name: String,
}

/// Activa un stack creando enlaces simbólicos y ejecutando daemon-reload
pub async fn activate_stack(stack_name: &str) -> Result<Vec<String>> {
    let quadlet_dir = get_quadlet_dir();
    let git_repo_dir = get_git_repo_dir();
    let available_path = git_repo_dir
        .join(".config/containers/available")
        .join(stack_name);
    let systemd_path = quadlet_dir.clone();

    if !available_path.exists() {
        return Err(anyhow::anyhow!(
            "Stack '{}' not found in available stacks",
            stack_name
        ));
    }

    let mut created_links = vec![];
    let mut entries = tokio::fs::read_dir(&available_path).await?;

    // Crear enlaces simbólicos para todos los archivos quadlet
    while let Some(entry) = entries.next_entry().await? {
        let file_path = entry.path();
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            match extension {
                "container" | "network" | "volume" | "kube" | "pod" | "image" => {
                    if let Some(filename) = file_path.file_name() {
                        let symlink_path = systemd_path.join(filename);

                        // Si ya existe un archivo/symlink, lo eliminamos primero
                        if symlink_path.exists() {
                            tokio::fs::remove_file(&symlink_path).await?;
                        }

                        // Crear el enlace simbólico
                        #[cfg(unix)]
                        {
                            tokio::fs::symlink(&file_path, &symlink_path).await?;
                        }
                        #[cfg(not(unix))]
                        {
                            // En sistemas no-Unix, copiar el archivo
                            tokio::fs::copy(&file_path, &symlink_path).await?;
                        }

                        if let Some(name) = filename.to_str() {
                            created_links.push(name.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Ejecutar daemon-reload después de crear los enlaces
    if !created_links.is_empty() {
        if let Err(e) = systemd::daemon_reload().await {
            eprintln!("Warning: Failed to execute daemon-reload: {}", e);
        }
    }

    Ok(created_links)
}

/// Desactiva un stack eliminando sus enlaces simbólicos y ejecutando daemon-reload
pub async fn deactivate_stack(stack_name: &str) -> Result<Vec<String>> {
    let quadlet_dir = get_quadlet_dir();
    let git_repo_dir = get_git_repo_dir();
    let available_path = git_repo_dir
        .join(".config/containers/available")
        .join(stack_name);

    let mut removed_links = vec![];

    // Obtener la lista de archivos que pertenecen a este stack
    if available_path.exists() {
        let mut entries = tokio::fs::read_dir(&available_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_path = entry.path();
            if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
                match extension {
                    "container" | "network" | "volume" | "kube" | "pod" | "image" => {
                        if let Some(filename) = file_path.file_name() {
                            let symlink_path = quadlet_dir.join(filename);

                            // Verificar si es un symlink apuntando a nuestro available stack
                            if symlink_path.exists() {
                                #[cfg(unix)]
                                {
                                    if let Ok(link_target) =
                                        tokio::fs::read_link(&symlink_path).await
                                    {
                                        if link_target == file_path {
                                            tokio::fs::remove_file(&symlink_path).await?;
                                            if let Some(name) = filename.to_str() {
                                                removed_links.push(name.to_string());
                                            }
                                        }
                                    }
                                }
                                #[cfg(not(unix))]
                                {
                                    // En sistemas no-Unix, simplemente eliminar el archivo si existe
                                    tokio::fs::remove_file(&symlink_path).await?;
                                    if let Some(name) = filename.to_str() {
                                        removed_links.push(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Ejecutar daemon-reload después de eliminar los enlaces
    if !removed_links.is_empty() {
        if let Err(e) = systemd::daemon_reload().await {
            eprintln!("Warning: Failed to execute daemon-reload: {}", e);
        }
    }

    Ok(removed_links)
}

/// Lista los stacks que están actualmente activos (tienen symlinks en systemd)
pub async fn list_active_stacks() -> Result<Vec<ActiveStack>> {
    let quadlet_dir = get_quadlet_dir();
    let git_repo_dir = get_git_repo_dir();
    let available_path = git_repo_dir.join(".config/containers/available");

    if !available_path.exists() {
        return Ok(vec![]);
    }

    let mut active_stacks = std::collections::HashMap::<String, ActiveStack>::new();
    let mut entries = tokio::fs::read_dir(&quadlet_dir).await?;

    // Revisar todos los archivos en el directorio systemd
    while let Some(entry) = entries.next_entry().await? {
        let symlink_path = entry.path();
        if let Some(extension) = symlink_path.extension().and_then(|e| e.to_str()) {
            match extension {
                "container" | "network" | "volume" | "kube" | "pod" | "image" => {
                    #[cfg(unix)]
                    {
                        // Verificar si es un symlink y hacia dónde apunta
                        if let Ok(link_target) = tokio::fs::read_link(&symlink_path).await {
                            // Verificar si apunta hacia available/
                            if let Some(target_str) = link_target.to_str() {
                                if target_str.contains(".config/containers/available/") {
                                    // Extraer el nombre del stack desde la ruta del target
                                    let path_parts: Vec<&str> = target_str.split('/').collect();
                                    if let Some(available_idx) =
                                        path_parts.iter().position(|&x| x == "available")
                                    {
                                        if available_idx + 1 < path_parts.len() {
                                            let stack_name =
                                                path_parts[available_idx + 1].to_string();
                                            let filename = symlink_path
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("")
                                                .to_string();

                                            active_stacks
                                                .entry(stack_name.clone())
                                                .or_insert_with(|| ActiveStack {
                                                    name: stack_name.clone(),
                                                    files: vec![],
                                                    source_path: git_repo_dir
                                                        .join(".config/containers/available")
                                                        .join(&stack_name)
                                                        .to_string_lossy()
                                                        .to_string(),
                                                })
                                                .files
                                                .push(filename);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(active_stacks.into_values().collect())
}
