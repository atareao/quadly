use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use anyhow::{anyhow, Result};

#[derive(Parser)]
#[grammar = "core/quadlet.pest"]
pub struct QuadletParser;

/// Parsea el contenido de un archivo .container a una estructura de datos
pub fn parse_quadlet(content: &str) -> Result<HashMap<String, HashMap<String, String>>> {
    let file = QuadletParser::parse(Rule::file, content)
        .map_err(|e| anyhow!("Error de sintaxis: {}", e))?
        .next()
        .ok_or_else(|| anyhow!("Archivo vacío o inválido"))?;

    let mut data: HashMap<String, HashMap<String, String>> = HashMap::new();
    for record in file.into_inner() {
        match record.as_rule() {
            Rule::section => {
                let mut inner = record.into_inner();
                let section_name = inner.next().unwrap().as_str().to_string();
                
                // Usamos entry para obtener o crear la sección
                let section_map = data.entry(section_name).or_default();

                for pair in inner {
                    if pair.as_rule() == Rule::pair {
                        let mut pair_inner = pair.into_inner();
                        let key = pair_inner.next().unwrap().as_str().to_string();
                        let value = pair_inner.next().unwrap().as_str().trim().to_string();

                        // Manejo de claves duplicadas (ej: Volume=...)
                        section_map.entry(key)
                            .and_modify(|old| {
                                if !old.is_empty() { old.push_str(", "); }
                                old.push_str(&value);
                            })
                            .or_insert(value);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(data)
}

/// Convierte el mapa de datos de nuevo a formato string .container
pub fn serialize_quadlet(data: &HashMap<String, HashMap<String, String>>) -> String {
    let mut output = String::new();
    for (section, pairs) in data {
        output.push_str(&format!("[{}]\n", section));
        for (key, value) in pairs {
            // Si el valor tiene comas (claves múltiples), las separamos al escribir
            for val in value.split(", ") {
                output.push_str(&format!("{}={}\n", key, val));
            }
        }
        output.push_str("\n");
    }
    output
}
