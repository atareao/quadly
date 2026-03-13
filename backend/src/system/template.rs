use crate::models::{
    Config, OutputDestination, ProcessStackRequest, ProcessStackResult, ProcessedTemplate,
    StackMetadata, TemplateStack,
};
use anyhow::{Context, Result};
use git2::{Repository, RepositoryOpenFlags, ResetType};
use minijinja::{Environment, Value};
use serde_json::Map;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// Gestor del sistema de plantillas
pub struct TemplateManager {
    templates_dir: PathBuf,
}

impl TemplateManager {
    /// Crea una nueva instancia del gestor de plantillas
    pub fn new() -> Self {
        let templates_dir = std::env::temp_dir().join("quadly-templates");
        Self { templates_dir }
    }

    /// Crea una instancia con directorio personalizado
    pub fn with_directory<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            templates_dir: dir.as_ref().to_path_buf(),
        }
    }

    /// Clona o actualiza el repositorio de plantillas
    pub async fn sync_templates(&self, pool: &sqlx::SqlitePool) -> Result<()> {
        let url = Config::get_quadly_templates_url(pool)
            .await
            .context("Error obteniendo URL de templates de la base de datos")?
            .ok_or_else(|| anyhow::anyhow!("URL de templates no configurada en la base de datos. Configure 'url_quadly_templates' en la tabla config"))?;

        info!("Sincronizando templates desde: {}", url);

        if self.templates_dir.exists() {
            info!("Directorio de templates existe, actualizando...");
            self.update_repository(&url).await?;
        } else {
            info!("Directorio de templates no existe, clonando...");
            self.clone_repository(&url).await?;
        }

        Ok(())
    }

    /// Clona el repositorio por primera vez
    async fn clone_repository(&self, url: &str) -> Result<()> {
        info!("Clonando repositorio de templates...");

        // Crear directorio padre si no existe
        if let Some(parent) = self.templates_dir.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Error creando directorio padre para templates: {:?}",
                    parent
                )
            })?;
        }

        Repository::clone(url, &self.templates_dir)
            .with_context(|| {
                format!(
                    "Error clonando repositorio '{}' a '{:?}'. Verifique que el repositorio existe y es accesible.",
                    url, self.templates_dir
                )
            })?;

        info!("Repositorio clonado exitosamente");
        Ok(())
    }

    /// Actualiza el repositorio existente
    async fn update_repository(&self, _url: &str) -> Result<()> {
        info!("Actualizando repositorio de templates...");

        let repo = Repository::open_ext(
            &self.templates_dir,
            RepositoryOpenFlags::empty(),
            &[] as &[&Path],
        )
        .context("Error abriendo repositorio existente")?;

        // Reset hard a HEAD para limpiar cambios locales
        let head = repo.head().context("Error obteniendo HEAD")?;
        let oid = head.target().context("Error obteniendo OID de HEAD")?;
        let commit = repo.find_commit(oid).context("Error encontrando commit")?;

        repo.reset(&commit.as_object(), ResetType::Hard, None)
            .context("Error reseteando repositorio")?;

        // Fetch y pull
        let mut remote = repo
            .find_remote("origin")
            .context("Error encontrando remote origin")?;

        remote
            .fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)
            .context("Error haciendo fetch")?;

        info!("Repositorio actualizado exitosamente");
        Ok(())
    }

    /// Lista todos los stacks disponibles
    pub async fn list_stacks(&self) -> Result<Vec<TemplateStack>> {
        let mut stacks = Vec::new();

        if !self.templates_dir.exists() {
            warn!("Directorio de templates no existe, retornando lista vacía");
            return Ok(stacks);
        }

        for entry in fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Saltar archivos que no son directorios
            if !path.is_dir() {
                continue;
            }

            // Saltar directorios especiales
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if dir_name.starts_with('.') || dir_name == "common" {
                continue;
            }

            // Buscar metadata.toml
            let metadata_path = path.join("metadata.toml");
            if !metadata_path.exists() {
                debug!("Directorio {} no tiene metadata.toml, saltando", dir_name);
                continue;
            }

            match self.load_stack_metadata(&metadata_path, dir_name).await {
                Ok(stack) => stacks.push(stack),
                Err(e) => {
                    error!("Error cargando metadata para {}: {}", dir_name, e);
                }
            }
        }

        info!("Encontrados {} stacks", stacks.len());
        Ok(stacks)
    }

    /// Carga los metadatos de un stack específico
    async fn load_stack_metadata(
        &self,
        metadata_path: &Path,
        stack_name: &str,
    ) -> Result<TemplateStack> {
        let content = fs::read_to_string(metadata_path).context("Error leyendo metadata.toml")?;

        let metadata: StackMetadata =
            toml::from_str(&content).context("Error parseando metadata.toml")?;

        let stack_dir = metadata_path.parent().unwrap();

        // Encontrar archivos de plantillas y estáticos
        let mut templates = Vec::new();
        let mut static_files = Vec::new();

        for entry in WalkDir::new(stack_dir).max_depth(1) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if filename == "metadata.toml" {
                    continue;
                }

                if filename.ends_with(".j2") {
                    templates.push(filename.to_string());
                } else {
                    static_files.push(filename.to_string());
                }
            }
        }

        Ok(TemplateStack {
            name: stack_name.to_string(),
            info: metadata.info,
            templates,
            static_files,
            outputs: metadata.outputs,
        })
    }

    /// Procesa un stack completo con las variables proporcionadas
    pub async fn process_stack(
        &self,
        request: ProcessStackRequest,
        pool: &sqlx::SqlitePool,
    ) -> Result<ProcessStackResult> {
        let stack_dir = self.templates_dir.join(&request.stack_name);
        let metadata_path = stack_dir.join("metadata.toml");

        if !metadata_path.exists() {
            return Err(anyhow::anyhow!(
                "Stack '{}' no encontrado",
                request.stack_name
            ));
        }

        let metadata = self
            .load_stack_metadata(&metadata_path, &request.stack_name)
            .await?;
        let common_dir = self.templates_dir.join("common");

        // Configurar entorno de Jinja2
        let mut env = Environment::new();
        env.set_loader(minijinja::path_loader(&stack_dir));

        // Cargar variables globales desde la base de datos
        self.setup_global_variables(&mut env, pool).await?;

        // Agregar common/ si existe
        if common_dir.exists() {
            env.add_global(
                "common_path",
                Value::from(common_dir.to_string_lossy().as_ref()),
            );
        }

        let mut result = ProcessStackResult {
            stack_name: request.stack_name.clone(),
            processed_templates: Vec::new(),
            copied_files: Vec::new(),
            errors: Vec::new(),
        };

        // Procesar plantillas
        for template_file in &metadata.templates {
            match self
                .process_single_template(
                    &env,
                    &stack_dir,
                    template_file,
                    &metadata.outputs,
                    &request.variables,
                    &request.target_directory,
                )
                .await
            {
                Ok(processed) => result.processed_templates.push(processed),
                Err(e) => {
                    error!("Error procesando plantilla {}: {}", template_file, e);
                    result
                        .errors
                        .push(format!("Error en {}: {}", template_file, e));
                }
            }
        }

        // Copiar archivos estáticos
        for static_file in &metadata.static_files {
            match self
                .copy_static_file(&stack_dir, static_file, &request.target_directory)
                .await
            {
                Ok(copied_path) => result.copied_files.push(copied_path),
                Err(e) => {
                    error!("Error copiando archivo estático {}: {}", static_file, e);
                    result
                        .errors
                        .push(format!("Error copiando {}: {}", static_file, e));
                }
            }
        }

        info!(
            "Stack '{}' procesado: {} plantillas, {} archivos estáticos, {} errores",
            request.stack_name,
            result.processed_templates.len(),
            result.copied_files.len(),
            result.errors.len()
        );

        Ok(result)
    }

    /// Procesa una plantilla individual
    async fn process_single_template(
        &self,
        env: &Environment<'_>,
        stack_dir: &Path,
        template_file: &str,
        outputs: &HashMap<String, String>,
        variables: &HashMap<String, serde_json::Value>,
        target_dir: &Option<String>,
    ) -> Result<ProcessedTemplate> {
        let template = env
            .get_template(template_file)
            .context("Error cargando plantilla")?;

        // Convertir variables JSON a contexto de minijinja
        let mut context = Map::new();
        for (key, value) in variables {
            context.insert(key.clone(), value.clone());
        }

        let rendered = template
            .render(&context)
            .context("Error renderizando plantilla")?;

        // Determinar destino y ruta de salida
        let destination = outputs
            .get(template_file)
            .cloned()
            .unwrap_or_else(|| "config".to_string());

        let output_filename = template_file.strip_suffix(".j2").unwrap_or(template_file);

        let output_path = if let Some(target) = target_dir {
            Path::new(target)
                .join(output_filename)
                .to_string_lossy()
                .to_string()
        } else {
            self.get_default_output_path(&destination, output_filename)?
        };

        Ok(ProcessedTemplate {
            template_name: template_file.to_string(),
            destination,
            content: rendered,
            output_path,
        })
    }

    /// Copia un archivo estático
    async fn copy_static_file(
        &self,
        _stack_dir: &Path,
        static_file: &str,
        target_dir: &Option<String>,
    ) -> Result<String> {
        let source_path = _stack_dir.join(static_file);

        let output_path = if let Some(target) = target_dir {
            Path::new(target)
                .join(static_file)
                .to_string_lossy()
                .to_string()
        } else {
            // Por defecto, copiar a un directorio de configuración
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("quadly")
                .join("configs");

            fs::create_dir_all(&config_dir).context("Error creando directorio de configuración")?;

            config_dir.join(static_file).to_string_lossy().to_string()
        };

        fs::copy(&source_path, &output_path).context("Error copiando archivo estático")?;

        Ok(output_path)
    }

    /// Determina la ruta de salida por defecto según el tipo de destino
    fn get_default_output_path(&self, destination: &str, filename: &str) -> Result<String> {
        match destination.parse::<OutputDestination>() {
            Ok(OutputDestination::Systemd) => {
                let systemd_dir = dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("containers")
                    .join("systemd");

                Ok(systemd_dir.join(filename).to_string_lossy().to_string())
            }
            Ok(OutputDestination::Config) => {
                let config_dir = dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("quadly")
                    .join("configs");

                Ok(config_dir.join(filename).to_string_lossy().to_string())
            }
            Err(_) => {
                // Destino desconocido, usar directorio de configuración genérico
                let config_dir = dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("quadly")
                    .join("configs");

                Ok(config_dir.join(filename).to_string_lossy().to_string())
            }
        }
    }

    /// Obtiene un stack específico por nombre
    pub async fn get_stack(&self, name: &str) -> Result<Option<TemplateStack>> {
        let stack_dir = self.templates_dir.join(name);
        let metadata_path = stack_dir.join("metadata.toml");

        if !metadata_path.exists() {
            return Ok(None);
        }

        match self.load_stack_metadata(&metadata_path, name).await {
            Ok(stack) => Ok(Some(stack)),
            Err(e) => {
                error!("Error cargando stack {}: {}", name, e);
                Err(e)
            }
        }
    }

    /// Configura las variables globales del entorno de plantillas desde la base de datos
    async fn setup_global_variables(
        &self,
        env: &mut Environment<'_>,
        pool: &sqlx::SqlitePool,
    ) -> Result<()> {
        debug!("Cargando variables globales desde la base de datos");

        // Mapeo de claves de configuración a nombres de variables en las plantillas
        let global_configs = [
            ("certresolver", "DEFAULT_CERTRESOLVER"),
            ("middlewares", "DEFAULT_MIDDLEWARES"),
            ("network", "DEFAULT_NETWORK"),
        ];

        // Cargar cada configuración y agregarla como variable global
        for (config_key, global_var) in &global_configs {
            match Config::get_by_key(pool, config_key).await {
                Ok(Some(config)) => {
                    env.add_global(*global_var, Value::from(config.value.clone()));
                    debug!(
                        "Variable global '{}' configurada con valor: {}",
                        global_var, config.value
                    );
                }
                Ok(None) => {
                    warn!(
                        "Configuración '{}' no encontrada en base de datos",
                        config_key
                    );
                }
                Err(e) => {
                    error!("Error cargando configuración '{}': {}", config_key, e);
                }
            }
        }

        // Agregar dominio base si existe
        if let Ok(Some(base_domain)) = Config::get_by_key(pool, "base_domain").await {
            env.add_global("BASE_DOMAIN", Value::from(base_domain.value.clone()));
            debug!(
                "Variable global 'BASE_DOMAIN' configurada con valor: {}",
                base_domain.value
            );
        }

        Ok(())
    }
}
