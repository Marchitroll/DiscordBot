use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use std::fs;

const CONFIG_FILE: &str = "confession_channels.txt";

fn get_config_path() -> std::path::PathBuf {
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.push(CONFIG_FILE);
        return path;
    }
    std::path::PathBuf::from(CONFIG_FILE)
}

pub fn save_config_to_path(path: &std::path::Path, mod_id: u64, public_id: u64) -> Result<(), std::io::Error> {
    fs::write(path, format!("{}\n{}\n", mod_id, public_id))
}

pub fn save_config_file(mod_id: u64, public_id: u64) -> Result<(), std::io::Error> {
    save_config_to_path(&get_config_path(), mod_id, public_id)
}

pub fn load_config_from_path(path: &std::path::Path) -> Option<(u64, u64)> {
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let mod_id = lines.next()?.trim().parse::<u64>().ok()?;
    let public_id = lines.next()?.trim().parse::<u64>().ok()?;
    Some((mod_id, public_id))
}

pub fn load_config_file() -> Option<(u64, u64)> {
    load_config_from_path(&get_config_path())
}

pub fn delete_config_file() {
    let path = get_config_path();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}


/// Configura los canales para el sistema de confesiones.
#[poise::command(
    slash_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn confesiones_setup(
    ctx: Context<'_>,
    #[description = "Canal de moderación existente (opcional)"]
    canal_moderacion: Option<serenity::ChannelId>,
    #[description = "Canal público existente (opcional)"]
    canal_publico: Option<serenity::ChannelId>,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.send(
                poise::CreateReply::default()
                    .content("❌ Este comando solo se puede usar en un servidor.")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    };

    let everyone_id = serenity::RoleId::new(guild_id.get());

    let existing_channels = guild_id.channels(ctx.http()).await.unwrap_or_default();

    let find_text_channel = |name: &str| -> Option<serenity::ChannelId> {
        existing_channels
            .values()
            .find(|c| c.name == name && c.kind == serenity::ChannelType::Text)
            .map(|c| c.id)
    };

pub const DEFAULT_MOD_CHANNEL_NAME: &str = "confesiones-mod";
pub const DEFAULT_PUBLIC_CHANNEL_NAME: &str = "confesiones";

    let mod_channel_id = match canal_moderacion {
        Some(id) => id,
        None => match find_text_channel(DEFAULT_MOD_CHANNEL_NAME) {
            Some(id) => id,
            None => {
                let bot_id = ctx.framework().bot_id;
                let mod_overwrites = vec![
                    serenity::PermissionOverwrite {
                        allow: serenity::Permissions::empty(),
                        deny: serenity::Permissions::VIEW_CHANNEL,
                        kind: serenity::PermissionOverwriteType::Role(everyone_id),
                    },
                    serenity::PermissionOverwrite {
                        allow: serenity::Permissions::VIEW_CHANNEL
                            | serenity::Permissions::SEND_MESSAGES
                            | serenity::Permissions::EMBED_LINKS,
                        deny: serenity::Permissions::empty(),
                        kind: serenity::PermissionOverwriteType::Member(bot_id),
                    },
                ];

                guild_id
                    .create_channel(
                        ctx.http(),
                        serenity::CreateChannel::new(DEFAULT_MOD_CHANNEL_NAME)
                            .kind(serenity::ChannelType::Text)
                            .topic("Canal de moderación de confesiones. Aprueba, edita o rechaza confesiones anónimas.")
                            .permissions(mod_overwrites),
                    )
                    .await?
                    .id
            }
        },
    };

    let public_channel_id = match canal_publico {
        Some(id) => id,
        None => match find_text_channel(DEFAULT_PUBLIC_CHANNEL_NAME) {
            Some(id) => id,
            None => {
                let bot_id = ctx.framework().bot_id;
                let public_overwrites = vec![
                    serenity::PermissionOverwrite {
                        allow: serenity::Permissions::VIEW_CHANNEL,
                        deny: serenity::Permissions::SEND_MESSAGES,
                        kind: serenity::PermissionOverwriteType::Role(everyone_id),
                    },
                    serenity::PermissionOverwrite {
                        allow: serenity::Permissions::VIEW_CHANNEL
                            | serenity::Permissions::SEND_MESSAGES
                            | serenity::Permissions::EMBED_LINKS,
                        deny: serenity::Permissions::empty(),
                        kind: serenity::PermissionOverwriteType::Member(bot_id),
                    },
                ];

                guild_id
                    .create_channel(
                        ctx.http(),
                        serenity::CreateChannel::new(DEFAULT_PUBLIC_CHANNEL_NAME)
                            .kind(serenity::ChannelType::Text)
                            .topic("Tus confesiones son anónimas. Si fue editada por moderación, los cambios se indicarán al pie. Recuerda ser respetuoso.")
                            .permissions(public_overwrites),
                    )
                    .await?
                    .id
            }
        },
    };

    // Verificar si el bot tiene acceso a los canales configurados
    let mut access_warnings = Vec::new();

    if let Err(e) = mod_channel_id.to_channel(ctx.http()).await {
        if matches!(crate::commands::confession::classify_channel_error(&e), crate::commands::confession::ChannelErrorKind::MissingPermissions) {
            access_warnings.push(format!(
                "⚠️ **Atención**: El bot no tiene acceso al canal de moderación <#{}>. Como es un canal privado preexistente, debes añadir al bot manualmente en los permisos del canal.",
                mod_channel_id
            ));
        }
    }

    if let Err(e) = public_channel_id.to_channel(ctx.http()).await {
        if matches!(crate::commands::confession::classify_channel_error(&e), crate::commands::confession::ChannelErrorKind::MissingPermissions) {
            access_warnings.push(format!(
                "⚠️ **Atención**: El bot no tiene acceso al canal público <#{}>. Asegúrate de permitirle ver y enviar mensajes en los permisos del canal.",
                public_channel_id
            ));
        }
    }

    // Guardar en Data
    {
        let data = ctx.data();
        let mut config_guard = data.confession_config.write().await;
        *config_guard = Some(crate::commands::confession::ConfessionConfig {
            mod_channel: mod_channel_id,
            public_channel: public_channel_id,
        });
    }

    // Persistir en archivo plano
    if let Err(e) = save_config_file(mod_channel_id.get(), public_channel_id.get()) {
        tracing::error!("Error al guardar la configuración en archivo: {}", e);
    }

    let status_msg = if access_warnings.is_empty() {
        format!(
            "✅ Sistema de confesiones configurado con éxito:\n- Canal de Moderación: <#{}>\n- Canal Público: <#{}>",
            mod_channel_id, public_channel_id
        )
    } else {
        format!(
            "✅ Configuración guardada:\n- Canal de Moderación: <#{}>\n- Canal Público: <#{}>\n\n{}",
            mod_channel_id, public_channel_id, access_warnings.join("\n")
        )
    };

    ctx.send(
        poise::CreateReply::default()
            .content(status_msg)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_config_success() {
        // Arrange
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let mod_id = 123456789012345678u64;
        let public_id = 987654321098765432u64;

        // Act
        save_config_to_path(path, mod_id, public_id).expect("Guardado exitoso");
        let result = load_config_from_path(path);

        // Assert
        assert_eq!(result, Some((mod_id, public_id)));
    }

    #[test]
    fn test_load_corrupted_config() {
        // Arrange
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        std::fs::write(path, "archivo\ncorrupto\n").unwrap();

        // Act
        let result = load_config_from_path(path);

        // Assert
        assert_eq!(result, None);
    }

    #[test]
    fn test_load_incomplete_config() {
        // Arrange
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        std::fs::write(path, "123456789\n").unwrap();

        // Act
        let result = load_config_from_path(path);

        // Assert
        assert_eq!(result, None);
    }

    #[test]
    fn test_load_nonexistent_file() {
        // Arrange
        let path = std::path::Path::new("non_existent_file_xyz_12345.txt");

        // Act
        let result = load_config_from_path(path);

        // Assert
        assert_eq!(result, None);
    }
}

