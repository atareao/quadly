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
    pub content: Option<String>,
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
    pub fn new(name: &str, extension: &str, content: Option<String>) -> Result<Self, std::io::Error> {
        let kind = QuadletType::from_extension(&extension).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported Quadlet type"))?;
        Ok(Self {
            name: name.to_string(),
            kind,
            description: None,
            content,
            status: None,
        })
    }
    /// Devuelve el nombre completo del archivo (con extensión)
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.name, self.kind.as_str())
    }

    /// Devuelve la ruta completa del archivo en el sistema
    pub fn path(&self) -> PathBuf {
        get_quadlet_dir().join(format!("{}.{}", self.name, self.kind.as_str()))
    }

    /// Salva el contenido del Quadlet en el sistema de archivos. Si el Quadlet no tiene contenido, devuelve un error.
    pub async fn save(&self) -> std::io::Result<()> {
        if self.content.is_none() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Quadlet can not be saved without content"));
        }
        tokio::fs::write(self.path(), &self.content.clone().unwrap()).await
    }

    /// Reads the content of the Quadlet from the file system and updates the `content` field. If the file does not exist or cannot be read, returns an error.
    pub async fn read(&mut self) -> std::io::Result<()> {
        self.content = Some(tokio::fs::read_to_string(self.path()).await?);
        Ok(())
    }

    pub async fn delete(&self) -> std::io::Result<()> {
        tokio::fs::remove_file(self.path()).await
    }

    pub async fn read_by_extension_and_name(extension: &str, name: &str) -> std::io::Result<String> {
        let path = get_quadlet_dir().join(format!("{}.{}", name, extension));
        tokio::fs::read_to_string(path).await
    }

    pub async fn read_by_extension(extension: &str) -> std::io::Result<Vec<Self>> {
        if QuadletType::allowed_extensions().iter().find(|&&ext| ext == extension).is_none() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unsupported Quadlet type"));
        }
        let dir = get_quadlet_dir();
        let mut quadlets = Vec::new();
        if let Ok(entries) = tokio::fs::read_dir(&dir).await {
            let mut entries = entries;
            while let Some(entry) = entries.next_entry().await? {
                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_file() {
                        if let Some(name_with_extension) = entry.file_name().to_str() {
                            if name_with_extension.ends_with(extension) {
                                let mut quadlet = Quadlet::new(
                                    &name_with_extension.trim_end_matches(extension).trim_end_matches('.').to_string(),
                                    &extension.trim_start_matches('.').to_string(),
                                    None,
                                ).unwrap();
                                quadlet.read().await?;
                                quadlets.push(quadlet);
                            }
                        }
                    }
                }
            }
        }
        Ok(quadlets)
    }

}


