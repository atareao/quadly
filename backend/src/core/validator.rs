use serde::Serialize;
use ts_rs::TS;
use std::collections::HashMap;

#[derive(Serialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/ValidationError.ts")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

pub struct SemanticValidator;

impl SemanticValidator {
    pub fn validate(parsed_data: &HashMap<String, HashMap<String, String>>) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // 1. Validar existencia de la sección [Container]
        if let Some(container_section) = parsed_data.get("Container") {
            
            // 2. Validar campo obligatorio: Image
            if !container_section.contains_key("Image") {
                errors.push(ValidationError {
                    field: "Container.Image".to_string(),
                    message: "La clave 'Image' es obligatoria para definir un contenedor.".to_string(),
                });
            }

            // 3. Validar formato de nombres (ejemplo: ContainerName)
            if let Some(name) = container_section.get("ContainerName") {
                if name.contains(' ') {
                    errors.push(ValidationError {
                        field: "Container.ContainerName".to_string(),
                        message: "El nombre del contenedor no puede contener espacios.".to_string(),
                    });
                }
            }
        } else {
            errors.push(ValidationError {
                field: "Global".to_string(),
                message: "No se encontró la sección obligatoria [Container].".to_string(),
            });
        }

        errors
    }
}
