use std::process::Command;
use anyhow::{Context, Result};

pub fn get_service_logs(name: &str, lines: u32) -> Result<String> {
    let unit_name = format!("{}.service", name);
    
    // Ejecutamos journalctl --user -u <nombre> -n <lineas> --no-pager
    let output = Command::new("journalctl")
        .arg("--user")
        .arg("-u")
        .arg(&unit_name)
        .arg("-n")
        .arg(lines.to_string())
        .arg("--no-pager") // Importante para que no se quede bloqueado esperando input
        .output()
        .context("Falló al ejecutar journalctl")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Error obteniendo logs: {}", error))
    }
}
