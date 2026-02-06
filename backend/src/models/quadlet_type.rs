use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Tipo de archivo Quadlet soportado
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../frontend/src/bindings/QuadletType.ts")]
pub enum QuadletType {
    Container,
    Network,
    Volume,
    Kube,
    Pod,
    Image,
}

impl QuadletType {
    /// Devuelve la extensión de archivo asociada a este tipo
    pub fn extension(&self) -> &'static str {
        match self {
            QuadletType::Container => ".container",
            QuadletType::Network => ".network",
            QuadletType::Pod => ".pod",
            QuadletType::Image => ".image",
            QuadletType::Volume => ".volume",
            QuadletType::Kube => ".kube",
        }
    }

    /// Intenta determinar el tipo de Quadlet desde una extensión de archivo
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            ".container" => Some(QuadletType::Container),
            ".network" => Some(QuadletType::Network),
            ".pod" => Some(QuadletType::Pod),
            ".image" => Some(QuadletType::Image),
            ".volume" => Some(QuadletType::Volume),
            ".kube" => Some(QuadletType::Kube),
            _ => None,
        }
    }

    /// Devuelve una representación en string del tipo
    pub fn as_str(&self) -> &'static str {
        match self {
            QuadletType::Container => "container",
            QuadletType::Network => "network",
            QuadletType::Pod => "pod",
            QuadletType::Image => "image",
            QuadletType::Volume => "volume",
            QuadletType::Kube => "kube",
        }
    }
}
