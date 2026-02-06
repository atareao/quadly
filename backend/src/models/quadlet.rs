use serde::{Deserialize, Serialize};
use ts_rs::TS;
use std::path::PathBuf;
use super::quadlet_type::QuadletType;

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/QuadletStatus.ts")]
pub enum QuadletStatus {
    Active,
    Inactive,
    Failed,
    Activating,
    Deactivating,
    Unknown,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/Quadlet.ts")]
pub struct Quadlet {
    /// Nombre del archivo (sin extensión)
    pub name: String,
    /// Tipo de Quadlet
    pub kind: QuadletType,
    /// Descripción breve del Quadlet (opcional)
    pub description: Option<String>,
    /// Contenido del archivo
    pub content: String,
    /// Ruta completa al archivo en el sistema de archivos
    pub path: PathBuf,
    /// Status actual del Quadlet
    pub status: Option<QuadletStatus>,
}

pub fn get_quadlet_dir() -> PathBuf {
    // Para modo --user: ~/.config/containers/systemd/
    let home = std::env::var("HOME").expect("No se pudo encontrar la variable HOME");
    PathBuf::from(home).join(".config/containers/systemd")
}

impl Quadlet {
    /// Crea una nueva instancia de Quadlet
    pub fn new(name: String, kind: QuadletType, content: String, path: PathBuf) -> Self {
        Self {
            name,
            kind,
            description: None,
            content,
            path,
            status: None,
        }
    }
    /// Devuelve el nombre completo del archivo (con extensión)
    pub fn full_name(&self) -> String {
        format!("{}{}", self.name, self.kind.extension())
    }

    pub async fn save(&self) -> std::io::Result<()> {
        let dir = get_quadlet_dir();
        let extension = self.kind.extension();
        let path = dir.join(format!("{}{}", self.name, extension));
        tokio::fs::create_dir_all(&dir).await?; // Aseguramos que el directorio exista
        tokio::fs::write(path, &self.content).await
    }

    pub async fn read(path: PathBuf) -> std::io::Result<Self> {
        let (name, extension) = match path.file_name().and_then(|s| s.to_str()).and_then(|s| s.rsplit_once('.')) {
            Some((n, e)) => (n, format!(".{}", e)),
            None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Nombre de archivo inválido")),
        };
        let content = tokio::fs::read_to_string(&path).await?;
        let kind = QuadletType::from_extension(&extension).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Tipo de Quadlet desconocido"))?;
        Ok(Self::new(name.to_string(), kind, content, path))
    }

    pub async fn read_by_type(type_filter: QuadletType) -> std::io::Result<Vec<Self>> {
        let dir = get_quadlet_dir();
        let mut quadlets = Vec::new();
        if let Ok(entries) = tokio::fs::read_dir(&dir).await {
            let mut entries = entries;
            while let Some(entry) = entries.next_entry().await? {
                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_file() {
                        if let Some(name_with_extension) = entry.file_name().to_str() {
                            if name_with_extension.ends_with(type_filter.extension()) {
                                if let Ok(quadlet) = Self::read(entry.path()).await {
                                    quadlets.push(quadlet);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(quadlets)
    }

    pub async fn read_by_type_name(type_name: &str) -> std::io::Result<Vec<Self>> {
        let quadlet_type = QuadletType::from_extension(&format!(".{}", type_name)).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Tipo de Quadlet desconocido"))?;
        Self::read_by_type(quadlet_type).await
    }

    pub async fn read_all() -> std::io::Result<Vec<Self>> {
        let dir = get_quadlet_dir();
        let mut quadlets = Vec::new();
        if let Ok(entries) = tokio::fs::read_dir(&dir).await {
            let mut entries = entries;
            while let Some(entry) = entries.next_entry().await? {
                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_file() {
                        if let Some(name_with_extension) = entry.file_name().to_str() {
                            if let Ok(quadlet) = Self::read(entry.path()).await {
                                quadlets.push(quadlet);
                            }
                        }
                    }
                }
            }
        }
        Ok(quadlets)
    }

    pub async fn delete(&self) -> std::io::Result<()> {
        let dir = get_quadlet_dir();
        let extension = self.kind.extension();
        let path = dir.join(format!("{}{}", self.name, extension));
        tokio::fs::remove_file(path).await
    }
    
}


