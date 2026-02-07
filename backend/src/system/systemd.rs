use crate::models::{Quadlet, QuadletStatus};
use anyhow::Result;
use futures_util::StreamExt;
use zbus::{Connection, fdo::PropertiesProxy, proxy};

// Proxy para el Manager de systemd
#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait SystemdManager {
    /// Método para obtener la ruta del objeto de una unidad específica
    fn get_unit(&self, name: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn restart_unit(&self, name: &str, mode: &str)
    -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn reload(&self) -> zbus::Result<()>;
}

// Proxy para la Unidad individual
#[proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
trait SystemdUnit {
    /// Propiedad que indica el estado de activación (active, inactive, failed, etc.)
    #[zbus(property)]
    fn active_state(&self) -> zbus::Result<String>;

    /// Propiedad que indica el estado de carga (loaded, not-found, etc.)
    #[zbus(property)]
    fn load_state(&self) -> zbus::Result<String>;
}

/// Función principal para obtener el estado de un Quadlet
pub async fn get_status(name: &str) -> QuadletStatus {
    // Los Quadlets generan servicios con el sufijo .service
    let unit_name = format!("{}.service", name);

    let result = async {
        // Conexión al bus de sesión (rootless)
        let conn = Connection::session().await?;
        let manager = SystemdManagerProxy::new(&conn).await?;

        // 1. Obtener la ruta de la unidad
        let unit_path = manager.get_unit(&unit_name).await?;

        // 2. Crear un proxy para esa unidad específica
        let unit = SystemdUnitProxy::builder(&conn)
            .path(unit_path)?
            .build()
            .await?;

        // 3. Consultar la propiedad ActiveState
        let state = unit.active_state().await?;

        Ok::<QuadletStatus, zbus::Error>(match state.as_str() {
            "active" | "reloading" | "activating" => QuadletStatus::Active,
            "inactive" | "deactivating" => QuadletStatus::Inactive,
            "failed" => QuadletStatus::Failed,
            _ => QuadletStatus::Unknown,
        })
    }
    .await;

    // Si hay un error (ej. la unidad no existe), devolvemos Inactive o Unknown
    result.unwrap_or(QuadletStatus::Inactive)
}

pub async fn monitor_systemd_events(tx: tokio::sync::broadcast::Sender<Quadlet>) -> Result<()> {
    let conn = Connection::session().await?;

    // Nos suscribimos a los cambios de propiedades del Manager de systemd
    let proxy = PropertiesProxy::builder(&conn)
        .destination("org.freedesktop.systemd1")?
        .path("/org/freedesktop/systemd1")?
        .build()
        .await?;

    let mut stream = proxy.receive_properties_changed().await?;

    while let Some(_change) = stream.next().await {
        // Aquí filtramos si el cambio es de una unidad que nos interesa
        // Por simplicidad, cuando algo cambia, re-escaneamos o enviamos el evento
        // En una versión pro, extraeríamos qué unidad cambió del cuerpo de la señal

        // Enviamos una señal de "refresco" al canal
        let _ = tx.send(Quadlet::new("any", "any", None).unwrap());
    }
    Ok(())
}

/// Ejecuta una acción de control sobre un Quadlet
pub async fn run_unit_action(name: &str, action: &str) -> Result<()> {
    let unit_name = format!("{}.service", name);
    let conn = Connection::session().await?;
    let manager = SystemdManagerProxy::new(&conn).await?;

    match action {
        "start" => {
            manager.start_unit(&unit_name, "replace").await?;
        }
        "stop" => {
            manager.stop_unit(&unit_name, "replace").await?;
        }
        "restart" => {
            manager.restart_unit(&unit_name, "replace").await?;
        }
        "daemon-reload" => {
            manager.reload().await?;
        }
        _ => return Err(anyhow::anyhow!("Acción no soportada: {}", action)),
    }
    Ok(())
}
