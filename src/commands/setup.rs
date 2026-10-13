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

pub fn delete_config_from_path(path: &std::path::Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

pub fn delete_config_file() {
    delete_config_from_path(&get_config_path());
}

pub async fn check_channel_access(
    http: &serenity::Http,
    mod_id: serenity::ChannelId,
    pub_id: serenity::ChannelId,
) -> Vec<String> {
    let mut access_warnings = Vec::new();

    if let Err(e) = mod_id.to_channel(http).await {
        if matches!(crate::commands::confession::classify_channel_error(&e), crate::commands::confession::ChannelErrorKind::MissingPermissions) {
            access_warnings.push(format!(
                "⚠️ **Atención**: El bot no tiene acceso al canal de moderación <#{}>. Como es un canal privado, debes añadir al bot en los permisos del canal con acceso para ver y enviar mensajes.",
                mod_id
            ));
        } else if matches!(crate::commands::confession::classify_channel_error(&e), crate::commands::confession::ChannelErrorKind::UnknownChannel) {
            access_warnings.push(format!(
                "❌ **Error**: El canal de moderación <#{}> no existe o fue eliminado.",
                mod_id
            ));
        }
    }

    if let Err(e) = pub_id.to_channel(http).await {
        if matches!(crate::commands::confession::classify_channel_error(&e), crate::commands::confession::ChannelErrorKind::MissingPermissions) {
            access_warnings.push(format!(
                "⚠️ **Atención**: El bot no tiene acceso al canal público <#{}>. Asegúrate de permitirle ver y enviar mensajes en los permisos del canal.",
                pub_id
            ));
        } else if matches!(crate::commands::confession::classify_channel_error(&e), crate::commands::confession::ChannelErrorKind::UnknownChannel) {
            access_warnings.push(format!(
                "❌ **Error**: El canal público <#{}> no existe o fue eliminado.",
                pub_id
            ));
        }
    }

    access_warnings
}

pub async fn apply_config(data: &crate::Data, mod_id: serenity::ChannelId, pub_id: serenity::ChannelId) {
    {
        let mut config_guard = data.confession_config.write().await;
        *config_guard = Some(crate::commands::confession::ConfessionConfig {
            mod_channel: mod_id,
            public_channel: pub_id,
        });
    }

    if let Err(e) = save_config_file(mod_id.get(), pub_id.get()) {
        tracing::error!("Error al guardar la configuración en archivo: {}", e);
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
    #[description = "Canal de moderación donde se revisarán las confesiones"]
    canal_moderacion: serenity::ChannelId,
    #[description = "Canal público donde se publicarán las confesiones aprobadas"]
    canal_publico: serenity::ChannelId,
) -> Result<(), Error> {
    if ctx.guild_id().is_none() {
        ctx.send(
            poise::CreateReply::default()
                .content("❌ Este comando solo se puede usar en un servidor.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if canal_moderacion == canal_publico {
        ctx.send(
            poise::CreateReply::default()
                .content("⚠️ El canal de moderación y el canal público no pueden ser el mismo canal. Las confesiones anónimas deben revisarse en un canal privado antes de publicarse.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let existing_config = {
        let config_guard = ctx.data().confession_config.read().await;
        config_guard.as_ref().map(|c| (c.mod_channel, c.public_channel))
    }.or_else(|| {
        load_config_file().map(|(m, p)| (serenity::ChannelId::new(m), serenity::ChannelId::new(p)))
    });

    let warnings = check_channel_access(ctx.http(), canal_moderacion, canal_publico).await;

    // Si ya existe una configuración previa, pedir confirmación antes de reescribir
    if let Some((old_mod, old_pub)) = existing_config {
        // Si los canales son idénticos a los actuales, informar sin sobreescribir
        if old_mod == canal_moderacion && old_pub == canal_publico {
            let msg = if warnings.is_empty() {
                format!(
                    "ℹ️ El sistema de confesiones ya está configurado exactamente con esos canales:\n- Canal de Moderación: <#{}>\n- Canal Público: <#{}>",
                    canal_moderacion, canal_publico
                )
            } else {
                format!(
                    "ℹ️ El sistema de confesiones ya está configurado con esos canales:\n- Canal de Moderación: <#{}>\n- Canal Público: <#{}>\n\n{}",
                    canal_moderacion, canal_publico, warnings.join("\n")
                )
            };
            ctx.send(poise::CreateReply::default().content(msg).ephemeral(true)).await?;
            return Ok(());
        }

        let confirm_id = format!("setup_confirm_{}", ctx.id());
        let cancel_id = format!("setup_cancel_{}", ctx.id());

        let confirm_btn = serenity::CreateButton::new(&confirm_id)
            .label("Confirmar y Reemplazar")
            .style(serenity::ButtonStyle::Danger)
            .emoji('✅');
        let cancel_btn = serenity::CreateButton::new(&cancel_id)
            .label("Cancelar")
            .style(serenity::ButtonStyle::Secondary)
            .emoji('❌');

        let row = serenity::CreateActionRow::Buttons(vec![confirm_btn, cancel_btn]);

        let prompt_text = format!(
            "⚠️ **Ya hay un setup hecho para el sistema de confesiones:**\n- **Canal de Moderación actual:** <#{}>\n- **Canal Público actual:** <#{}>\n\n¿Aún así deseas volverlo a hacer? Se eliminará la configuración anterior y se reemplazará por:\n- **Nuevo Canal de Moderación:** <#{}>\n- **Nuevo Canal Público:** <#{}>\n\n*Presiona **Confirmar y Reemplazar** para proceder o **Cancelar** para mantener la configuración actual.*",
            old_mod, old_pub, canal_moderacion, canal_publico
        );

        let reply_handle = ctx.send(
            poise::CreateReply::default()
                .content(prompt_text)
                .components(vec![row])
                .ephemeral(true),
        )
        .await?;

        // Esperar interacción del usuario (timeout de 60 segundos)
        let interaction = poise::serenity_prelude::ComponentInteractionCollector::new(ctx.serenity_context())
            .timeout(std::time::Duration::from_secs(60))
            .filter({
                let c_id = confirm_id.clone();
                let x_id = cancel_id.clone();
                let author_id = ctx.author().id;
                move |mci| mci.user.id == author_id && (mci.data.custom_id == c_id || mci.data.custom_id == x_id)
            })
            .await;

        match interaction {
            Some(mci) => {
                if mci.data.custom_id == confirm_id {
                    apply_config(ctx.data(), canal_moderacion, canal_publico).await;

                    let final_msg = if warnings.is_empty() {
                        format!(
                            "✅ Configuración actualizada con éxito (se reemplazó la anterior):\n- Canal de Moderación: <#{}>\n- Canal Público: <#{}>",
                            canal_moderacion, canal_publico
                        )
                    } else {
                        format!(
                            "✅ Configuración actualizada con éxito:\n- Canal de Moderación: <#{}>\n- Canal Público: <#{}>\n\n{}",
                            canal_moderacion, canal_publico, warnings.join("\n")
                        )
                    };

                    mci.create_response(
                        ctx.serenity_context(),
                        serenity::CreateInteractionResponse::UpdateMessage(
                            serenity::CreateInteractionResponseMessage::new()
                                .content(final_msg)
                                .components(vec![]),
                        ),
                    )
                    .await?;
                } else {
                    mci.create_response(
                        ctx.serenity_context(),
                        serenity::CreateInteractionResponse::UpdateMessage(
                            serenity::CreateInteractionResponseMessage::new()
                                .content("❌ Se ha cancelado la reconfiguración. Se conserva la configuración anterior.")
                                .components(vec![]),
                        ),
                    )
                    .await?;
                }
            }
            None => {
                let _ = reply_handle.edit(
                    ctx,
                    poise::CreateReply::default()
                        .content("⌛ Tiempo de espera agotado (60s). No se realizaron cambios y se mantiene la configuración anterior.")
                        .components(vec![]),
                ).await;
            }
        }

        return Ok(());
    }

    // Si no había configuración previa, se aplica directamente
    apply_config(ctx.data(), canal_moderacion, canal_publico).await;

    let status_msg = if warnings.is_empty() {
        format!(
            "✅ Sistema de confesiones configurado con éxito:\n- Canal de Moderación: <#{}>\n- Canal Público: <#{}>",
            canal_moderacion, canal_publico
        )
    } else {
        format!(
            "✅ Configuración guardada:\n- Canal de Moderación: <#{}>\n- Canal Público: <#{}>\n\n{}",
            canal_moderacion, canal_publico, warnings.join("\n")
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

/// Muestra la configuración actual de los canales de confesiones.
#[poise::command(
    slash_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn confesiones_config(ctx: Context<'_>) -> Result<(), Error> {
    if ctx.guild_id().is_none() {
        ctx.send(
            poise::CreateReply::default()
                .content("❌ Este comando solo se puede usar en un servidor.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let current_config = {
        let config_guard = ctx.data().confession_config.read().await;
        config_guard.as_ref().map(|c| (c.mod_channel, c.public_channel))
    }.or_else(|| {
        load_config_file().map(|(m, p)| (serenity::ChannelId::new(m), serenity::ChannelId::new(p)))
    });

    let Some((mod_channel_id, public_channel_id)) = current_config else {
        ctx.send(
            poise::CreateReply::default()
                .content("ℹ️ El sistema de confesiones no está configurado actualmente.\nPuedes configurarlo usando `/confesiones_setup`.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let warnings = check_channel_access(ctx.http(), mod_channel_id, public_channel_id).await;

    let mut msg = format!(
        "📋 **Configuración actual del sistema de confesiones:**\n- **Canal de Moderación:** <#{}>\n- **Canal Público:** <#{}>",
        mod_channel_id, public_channel_id
    );

    if !warnings.is_empty() {
        msg.push_str("\n\n");
        msg.push_str(&warnings.join("\n"));
    }

    msg.push_str("\n\n*(Para modificarla usa `/confesiones_setup` o para eliminarla usa `/confesiones_deshacer_setup`)*");

    ctx.send(
        poise::CreateReply::default()
            .content(msg)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Elimina la configuración del sistema de confesiones (el bot olvida los canales).
#[poise::command(
    slash_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn confesiones_deshacer_setup(ctx: Context<'_>) -> Result<(), Error> {
    deshacer_setup_impl(ctx).await
}

/// Comando alias para deshacer la configuración de confesiones.
#[poise::command(
    slash_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn deshacer_setup(ctx: Context<'_>) -> Result<(), Error> {
    deshacer_setup_impl(ctx).await
}

async fn deshacer_setup_impl(ctx: Context<'_>) -> Result<(), Error> {
    if ctx.guild_id().is_none() {
        ctx.send(
            poise::CreateReply::default()
                .content("❌ Este comando solo se puede usar en un servidor.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let data = ctx.data();
    let had_config = {
        let config_guard = data.confession_config.read().await;
        config_guard.is_some() || load_config_file().is_some()
    };

    if !had_config {
        ctx.send(
            poise::CreateReply::default()
                .content("ℹ️ No hay ninguna configuración de canales de confesiones guardada para deshacer.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    data.invalidate_confession_config().await;

    ctx.send(
        poise::CreateReply::default()
            .content("🗑️ Configuración eliminada con éxito: el bot ha olvidado los canales configurados.\nPuedes volver a configurarlos cuando desees con `/confesiones_setup`.")
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

    #[test]
    fn test_delete_config_success() {
        // Arrange
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        save_config_to_path(&path, 123456789, 987654321).expect("Guardado exitoso");
        assert!(path.exists());

        // Act
        delete_config_from_path(&path);

        // Assert
        assert!(!path.exists());
    }

    #[test]
    fn test_delete_config_nonexistent_file() {
        // Arrange
        let path = std::path::Path::new("non_existent_file_xyz_delete_99999.txt");

        // Act & Assert (debe no entrar en pánico ni fallar)
        delete_config_from_path(path);
    }

    #[tokio::test]
    async fn test_apply_config_and_invalidate() {
        let data = crate::Data {
            confession_config: tokio::sync::RwLock::new(None),
            confession_cooldowns: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            tictactoe_games: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        };

        let mod_id = serenity::ChannelId::new(123456);
        let pub_id = serenity::ChannelId::new(654321);

        apply_config(&data, mod_id, pub_id).await;

        {
            let guard = data.confession_config.read().await;
            assert!(guard.is_some());
            let config = guard.as_ref().unwrap();
            assert_eq!(config.mod_channel, mod_id);
            assert_eq!(config.public_channel, pub_id);
        }

        data.invalidate_confession_config().await;

        {
            let guard = data.confession_config.read().await;
            assert!(guard.is_none());
        }
    }
}
