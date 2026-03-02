# Gestión de Quadlets via Git

Quadly ahora incluye funcionalidad completa para gestionar tus quadlets usando git como sistema de control de versiones. Esto te permite trackear cambios, mantener un historial y colaborar en el desarrollo de tus configuraciones de contenedores.

## Endpoints de Git

### GET `/api/v1/git/status`

Obtiene el estado actual del repositorio git de quadlets.

**Respuesta:**

```json
{
  "is_repo": true,
  "branch": "main",
  "staged": ["wp-app-data.volume"],
  "modified": ["nginx.container"],
  "untracked": ["nuevo-service.container"],
  "commits_ahead": 2,
  "commits_behind": 0
}
```

### POST `/api/v1/git/init`

Inicializa un repositorio git en el directorio de quadlets (`~/.config/containers/systemd/`).

### POST `/api/v1/git/add`

Agrega archivos al staging area.

**Cuerpo:**

```json
{
  "files": ["wp-app-data.volume", "nginx.container"]
}
```

### POST `/api/v1/git/commit`

Hace commit de los cambios staged o de archivos específicos.

**Cuerpo:**

```json
{
  "message": "Update nginx configuration",
  "files": ["nginx.container"] // Opcional: si se omite, commitea todos los staged
}
```

**Respuesta:**

```json
{
  "commit_hash": "a1b2c3d4e5f6",
  "message": "Changes committed successfully"
}
```

### GET `/api/v1/git/history?limit=10`

Obtiene el historial de commits.

**Parámetros query:**

- `limit` (opcional): Número máximo de commits a devolver (default: 20)

**Respuesta:**

```json
[
  {
    "hash": "a1b2c3d4e5f6",
    "author": "usuario",
    "date": "2026-02-07T19:30:00+01:00",
    "message": "Update nginx configuration",
    "files": ["nginx.container"]
  }
]
```

### GET `/api/v1/git/diff?file=nginx.container`

Obtiene las diferencias de archivos.

**Parámetros query:**

- `file` (opcional): Archivo específico para ver diff. Si se omite, muestra diff de todos los archivos

**Respuesta:** Texto plano con el diff de git.

### POST `/api/v1/git/revert`

Revierte cambios a un archivo específico o a un commit.

**Revertir archivo:**

```json
{
  "file": "nginx.container"
}
```

**Revertir a commit:**

```json
{
  "commit_hash": "a1b2c3d4e5f6"
}
```

## Workflow Típico

### 1. Inicializar Repositorio

```bash
curl -X POST http://localhost:3000/api/v1/git/init
```

### 2. Verificar Estado

```bash
curl http://localhost:3000/api/v1/git/status
```

### 3. Agregar Archivos

```bash
curl -X POST http://localhost:3000/api/v1/git/add \
  -H "Content-Type: application/json" \
  -d '{"files": ["*.container", "*.volume"]}'
```

### 4. Hacer Commit

```bash
curl -X POST http://localhost:3000/api/v1/git/commit \
  -H "Content-Type: application/json" \
  -d '{"message": "Initial quadlet configuration"}'
```

### 5. Ver Historial

```bash
curl http://localhost:3000/api/v1/git/history?limit=5
```

## Casos de Uso

### Backup y Versionado

- Mantén un historial completo de cambios en tus quadlets
- Revierte fácilmente a configuraciones anteriores
- Trackea qué cambió y cuándo

### Colaboración en Equipo

- Comparte configuraciones de quadlets
- Mantén sincronizados múltiples entornos
- Revisa cambios antes de aplicarlos

### Desarrollo y Debugging

- Experimenta con configuraciones sabiendo que puedes revertir
- Compara diferentes versiones para identificar problemas
- Mantén ramas separadas para diferentes entornos

## Integración con Quadlets

La gestión git se integra perfectamente con la gestión de quadlets:

1. **Detección Automática**: Cada vez que guardas un quadlet, aparece en git status
2. **Filtrado Inteligente**: Los endpoints git solo muestran archivos de quadlet (_.container, _.volume, etc.)
3. **Workflow Unificado**: Puedes gestionar y versionar tus quadlets desde la misma interfaz

## TypeScript Types

Los bindings de TypeScript están disponibles en:

- `GitStatus` - Estado del repositorio
- `GitCommit` - Información de commits

```typescript
interface GitStatus {
  is_repo: boolean;
  branch: string | null;
  staged: string[];
  modified: string[];
  untracked: string[];
  commits_ahead: number;
  commits_behind: number;
}

interface GitCommit {
  hash: string;
  author: string;
  date: string;
  message: string;
  files: string[];
}
```
