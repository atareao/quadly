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
('url_quadly_templates', 'https://github.com/atareao/quadly-templates.git', 'URL del repositorio de templates para quadlets');

-- Insertar configuraciones adicionales para Traefik
INSERT INTO config (key, value, description) VALUES 
('certresolver', 'myresolver', 'Resolver de certificados por defecto para Traefik'),
('middlewares', 'security-headers@file,compression-headers@file', 'Middlewares por defecto para Traefik'),
('network', 'proxy.network', 'Red por defecto para contenedores proxy'),
('base_domain', 'servidorlinux.es', 'Dominio base para la infraestructura');

-- Trigger para actualizar automáticamente el campo updated_at
CREATE TRIGGER IF NOT EXISTS update_config_timestamp 
    AFTER UPDATE ON config
    FOR EACH ROW
BEGIN
    UPDATE config SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
