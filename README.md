# Bot de Discord en Rust

Bot de Discord desarrollado en Rust con Poise, Serenity y Tokio.

## Requisitos Previos

- Windows 10/11 con WSL2 (Ubuntu).
- Rust y Cargo instalados en WSL2 via rustup.
- Cuenta de Discord y aplicación registrada en Discord Developer Portal.

## Configuración Inicial

### 1. Aplicación de Discord

1. Crear una aplicación en el Discord Developer Portal.
2. En la pestaña Bot, generar el token de acceso. Desactivar todos los Privileged Gateway Intents.
3. En OAuth2 -> URL Generator, seleccionar los scopes `bot` y `applications.commands`. Otorgar permisos para enviar mensajes y usar slash commands. Copiar el enlace de invitación e incorporar el bot al servidor de pruebas.
4. Activar el modo desarrollador en la configuración de Discord y copiar el ID del servidor de pruebas.

### 2. Archivo de entorno

Crear el archivo `.env` en la raíz del proyecto con la siguiente estructura:

```env
DISCORD_TOKEN=tu_token
GUILD_ID=tu_guild_id
RUST_LOG=info
```

### 3. Entorno Linux (WSL2)

En la terminal de Ubuntu:

1. Instalar las herramientas de compilación C:
   ```bash
   sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev
   ```
2. Añadir Cargo al entorno de forma permanente:
   ```bash
   echo 'source "$HOME/.cargo/env"' >> ~/.bashrc
   source ~/.bashrc
   ```

## Flujo de Ejecución

1. Abrir la terminal de Windows e iniciar WSL:
   ```powershell
   wsl
   ```
2. Navegar al directorio del proyecto:
   ```bash
   cd /mnt/c/GitHub/DiscordBot
   ```
3. Ejecutar el proyecto:
   ```bash
   cargo run
   ```

Para detener el proceso, presionar Ctrl + C.

## Comandos Disponibles

| Comando | Descripción | Parámetros |
|:---|:---|:---|
| `/ping` | Verifica si el bot responde. | Ninguno |
| `/echo` | Repite el texto ingresado por el usuario. | `message` (cadena de texto) |
| `/http` | Muestra una imagen de estado HTTP rotando entre 5 proveedores. | `code` (número entero, ej. 404) |

## Principios de Diseño y Arquitectura

- Principio de menor privilegio: El desarrollo se ejecuta dentro de WSL2 para no alterar políticas de seguridad en Windows 11.
- KISS: Proyecto con dependencias necesarias y código estructurado de forma directa.
- YAGNI: Oclusión de código no requerido, librerías innecesarias o configuraciones prematuras.
- Semver 0.x: Uso de dependencias en versiones estables pre-1.0 (`poise 0.6.2`, `serenity 0.12.x`) gestionando actualizaciones manualmente.

## Solución de Problemas

- Error `os error 4551` en Windows: Ocurre cuando el control de aplicaciones de Windows bloquea ejecutables no firmados. La solución es ejecutar el proyecto dentro de WSL2.
- Error `linker cc not found` en WSL: Indica que falta el compilador de C. Se resuelve instalando `build-essential` via `apt`.
- Error `cargo: command not found` en WSL: Falta incluir la ruta de Cargo en la variable PATH. Se resuelve ejecutando `source "$HOME/.cargo/env"` o añadiéndolo a `~/.bashrc`.
