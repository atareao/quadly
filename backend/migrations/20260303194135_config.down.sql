-- Eliminar el trigger primero
DROP TRIGGER IF EXISTS update_config_timestamp;

-- Eliminar la tabla de configuración
DROP TABLE IF EXISTS config;ar