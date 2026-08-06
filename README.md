# Bot de Discord en Rust

Bot de Discord desarrollado en Rust utilizando Poise, Serenity y Tokio.

## Requisitos Previos

* Windows 10/11 con WSL2 (Ubuntu).
* Rust y Cargo instalados en WSL2 mediante rustup.
* Cuenta de Discord y aplicación registrada en el Discord Developer Portal.

## Configuración Inicial

### 1. Aplicación de Discord

1. Crear una aplicación en el Discord Developer Portal.
2. En la pestaña **Bot**, generar el token de acceso. Por el principio de menor privilegio, **desactivar todos los Privileged Gateway Intents** (Presence Intent, Server Members Intent, Message Content Intent). Este bot funciona mediante interacciones (Slash Commands) y no requiere leer el chat general.
3. En la pestaña **OAuth2 > URL Generator**, generar el enlace de invitación seleccionando los siguientes permisos:

   **Scopes**
   - `bot`
   - `applications.commands`

   **Permisos del bot (Bot Permissions)**
   - **Ver canales (Read Messages / View Channels)**: Requerido para acceder a los canales y despachar los mensajes y embeds.
   - **Enviar mensajes**: Requerido para publicar respuestas y confesiones en los canales configurados.
   - **Insertar enlaces (Embed Links)**: Requerido para dar formato estructurado a las confesiones e interfaces de moderación.
   - **Gestionar canales (Manage Channels)**: Requerido si se utiliza `/confesiones-setup` sin argumentos para crear automáticamente los canales `#confesiones` y `#confesiones-mod`.
   - **Gestionar roles / permisos (Manage Roles / Permissions)**: Requerido únicamente si se desea que `/confesiones-setup` aplique automáticamente las restricciones de acceso (ocultar `#confesiones-mod` a `@everyone`). En la API de Discord, asignar o modificar restricciones por canal (*permission overwrites*) requiere este permiso.

   > [!NOTE]
   > **Principio de menor privilegio**: Si prefieres no otorgar `Gestionar Roles` al bot, puedes crear y restringir los canales manualmente desde la aplicación de Discord y luego vincularlos usando `/confesiones-setup canal_moderacion:#tu-canal-mod canal_publico:#tu-canal-publico`.

4. Copiar la URL generada e invitar al bot al servidor.
5. Activar el **Modo Desarrollador** en Discord (Ajustes > Avanzado) y copiar el ID del servidor.

### 2. Archivo de Entorno

Crear el archivo `.env` en la raíz del proyecto con la siguiente estructura:

```env
DISCORD_TOKEN=tu_token
GUILD_ID=tu_guild_id
RUST_LOG=info
```

*(Nota de Seguridad: Los archivos `.env` y `confession_channels.txt` ya están incluidos en el archivo `.gitignore` del proyecto por defecto. Esto previene la exposición accidental de credenciales o de la topología del servidor en repositorios públicos).*

### 3. Entorno Linux (WSL2)

En la terminal de Ubuntu:

1. Instalar las herramientas de compilación:
   ```bash
   sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev
   ```
2. Añadir Cargo al entorno de forma permanente:
   ```bash
   echo 'source "$HOME/.cargo/env"' >> ~/.bashrc
   source ~/.bashrc
   ```

## Flujo de Ejecución

1. Abrir la terminal e iniciar WSL:
   ```powershell
   wsl
   ```
2. Navegar al directorio del proyecto:
   ```bash
   cd /mnt/c/GitHub/DiscordBot
   ```
3. (Opcional) Ejecutar las pruebas unitarias:
   ```bash
   cargo test
   ```
4. Ejecutar el bot:
   ```bash
   cargo run
   ```

Para detener el proceso, presionar `Ctrl + C`.

## Comandos Disponibles

| Comando | Descripción | Parámetros |
|:---|:---|:---|
| `/ping` | Verifica si el bot responde. | Ninguno |
| `/echo` | Repite el texto ingresado por el usuario. | `message` (cadena de texto) |
| `/http` | Muestra una imagen de estado HTTP rotando entre 5 proveedores. | `code` (número entero) |
| `/confesion` | Envía una confesión para revisión en moderación sin registrar el autor. | `texto` (cadena de texto) |
| `/confesiones-setup` | Configura los canales. Si se omiten parámetros, el bot **crea canales nuevos** automáticamente (requiere permisos de gestión). Si se especifican canales preexistentes, valida los permisos de acceso e informa al administrador de cualquier ajuste manual requerido. | `canal_moderacion` (opcional), `canal_publico` (opcional) |
| `/tictactoe` | Inicia una partida de 3 en raya contra el bot. | `dificultad` (opcional: Fácil/Difícil, por defecto Fácil) |

## Sistema de Confesiones

El bot implementa un flujo de moderación enfocado en la privacidad:

> [!WARNING]
> **Consideraciones sobre Privacidad**  
> Aunque la aplicación está diseñada para no registrar ni exponer la identidad de los autores a los administradores del servidor, toda comunicación a través de Discord está sujeta a la infraestructura y políticas de la plataforma, la cual procesa metadatos a nivel de red y transporte.

* **Sin persistencia de identidad**: El código no almacena registros (*logs*) que vinculen al remitente con el contenido de su mensaje en archivos locales ni en bases de datos.
* **Flujo de moderación**: Las confesiones enviadas con `/confesion` llegan al canal privado `#confesiones-mod` en un embed con botones interactivos:
  * **Aprobar**: Publica la confesión en el canal público `#confesiones` y deshabilita los botones.
  * **Editar**: Permite a un moderador corregir el texto, agregando una nota informativa al pie del mensaje público.
  * **Rechazar**: Marca la confesión como rechazada y deshabilita los botones.
* **Límite de frecuencia (Cooldown)**: Limita el envío a 1 confesión cada 5 minutos por usuario estándar (almacenado temporalmente en memoria RAM con limpieza de entradas expiradas para optimizar el uso de memoria). **Nota:** Administradores y moderadores del servidor están exentos de este límite.
  * *Persistencia en memoria*: Dado que el control de frecuencia reside en la memoria volátil del proceso, un reinicio del bot restablece los tiempos de espera a cero.
* **Persistencia de canales**: Los IDs de los canales configurados se guardan localmente en `confession_channels.txt`.

*(Para conocer más sobre el contexto y los riesgos comunes en este tipo de herramientas, consulta el [Contexto Histórico y Seguridad](docs/security-background.md)).*

## Principios de Diseño y Arquitectura

* **Privacidad por Diseño (Sin Persistencia de Autor)**: La aplicación desacopla el identificador del usuario del contenido de la confesión. La ID de Discord se utiliza temporalmente en memoria solo para validar el intervalo de envío (*cooldown* de 5 minutos) y no se persiste junto con el mensaje en archivos, bases de datos ni en los embeds enviados a Discord. De este modo, los moderadores y administradores del servidor no tienen acceso al remitente a través del bot.
* **Aislamiento de Entorno (Sandboxing)**: El desarrollo y compilación se realizan dentro de WSL2 (Ubuntu) para encapsular dependencias y evitar alteraciones en el entorno anfitrión (Windows).
* **KISS (Sin Dependencia de Bases de Datos)**: Al utilizar archivos de texto plano locales únicamente para los identificadores de canales, se reduce la complejidad de mantenimiento y la superficie de exposición asociada a servicios de bases de datos externos.
* **YAGNI**: Implementación acotada a los requerimientos del proyecto, priorizando pruebas unitarias sobre la lógica de dominio interno y evitando simuladores de red innecesariamente complejos.
* **SOLID**: Separación modular de responsabilidades (`main.rs` arranca la aplicación, `confession.rs` gestiona la lógica de dominio y `interactions.rs` canaliza eventos de interfaz).
* **Gestión de Dependencias**: Uso de dependencias pre-1.0 (`poise 0.6.2`, `serenity 0.12.x`) con seguimiento de versiones y análisis estático mediante `cargo audit`.

## Pruebas Unitarias

Se implementaron pruebas unitarias siguiendo los principios F.I.R.S.T., AAA y Hermeticidad, utilizando `tempfile` para aislamiento en disco y `serde_json` para verificar la estructura de componentes.

### Pruebas Implementadas

* **Lectura/Escritura Hermética**: Almacenamiento y carga de canales mediante inyección de rutas temporales.
* **Formatos de Embeds**: Verificación de estructura, colores, campos informativos y notas de edición acumuladas en estados Aprobado, Pendiente y Rechazado.
* **Manejo de Errores**: Validación ante archivos corruptos o inexistentes y detección de canales inaccesibles.
* **Componentes**: Verificación de estados y etiquetas de botones interactivos.

### Casos Excluidos de Pruebas Unitarias

* **Interacción con la API de Discord**: Las llamadas HTTP reales y validaciones remotas de Discord se excluyen de las pruebas unitarias locales (YAGNI).
* **Estado en Tiempo de Ejecución y Gateway**: La gestión de eventos en tiempo real mediante WebSocket se evalúa en el entorno de ejecución.
* **Ciclo de Vida E2E**: Eventos como reinicio del proceso o expulsión del bot del servidor se validan mediante pruebas funcionales directas.

## Solución de Problemas

* **Error `os error 4551` en Windows**: Bloqueo de ejecutables no firmados por el control de aplicaciones. Solución: ejecutar dentro de WSL2.
* **Error `linker cc not found` en WSL**: Falta de compilador de C. Solución: instalar `build-essential` mediante `apt`.
* **Error `cargo: command not found` en WSL**: Variable PATH no cargada. Solución: ejecutar `source "$HOME/.cargo/env"` o añadirlo a `~/.bashrc`.
* **Compilación lenta en WSL2**: Configurar `target-dir` hacia el sistema de archivos Linux (ext4) como `/tmp/discord-bot-target` en `.cargo/config.toml`.

## Propósito del Proyecto, Estado y Licencia

Este es un proyecto personal en constante evolución desarrollado para explorar la construcción de aplicaciones en Rust. No tiene fines comerciales ni intención de ofrecerse como un servicio público administrado.

### Licencia

El código fuente se comparte bajo los términos de la Licencia **MIT**. Puedes consultar los detalles en el archivo [LICENSE](LICENSE).
