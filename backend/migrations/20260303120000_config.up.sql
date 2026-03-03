CREATE TABLE IF NOT EXISTS config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL,
    description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insertar la configuración por defecto para url_quadly_templates
INSERT INTO config (key, value, description) VALUES 
('url_quadly_templates', 'https://github.com/quadly-org/quadlet-templates.git', 'URL del repositorio de templates para quadlets');

-- Trigger para actualizar automáticamente el campo updated_at
CREATE TRIGGER IF NOT EXISTS update_config_timestamp 
    AFTER UPDATE ON config
    FOR EACH ROW
BEGIN
    UPDATE config SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;