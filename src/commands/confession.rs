use crate::{Context, Error};
use poise::serenity_prelude as serenity;

pub const COLOR_PENDING: u32 = 0xF1C40F;
pub const COLOR_APPROVED: u32 = 0x2ECC71;
pub const COLOR_REJECTED: u32 = 0xE74C3C;
pub const COLOR_PUBLIC: u32 = 0x5B2C6F;

pub const FIELD_STATUS: &str = "Estado";
pub const FIELD_ORIGINAL_TEXT: &str = "Texto original";
pub const FIELD_EDIT_NOTES: &str = "Notas de edición";

pub const CONFESSION_COOLDOWN_SECS: u64 = 300;

#[derive(Debug)]
pub struct ConfessionConfig {
    pub mod_channel: serenity::ChannelId,
    pub public_channel: serenity::ChannelId,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfessionModData<'a> {
    pub text: &'a str,
    pub original_text: &'a str,
    pub edit_notes: Option<&'a str>,
}

pub fn extract_mod_embed_data(embed: &serenity::Embed) -> ConfessionModData<'_> {
    let text = embed.description.as_deref().unwrap_or("");
    let edit_notes = embed
        .fields
        .iter()
        .find(|f| f.name == FIELD_EDIT_NOTES)
        .map(|f| f.value.as_str());
    let original_text = embed
        .fields
        .iter()
        .find(|f| f.name == FIELD_ORIGINAL_TEXT || f.name == "__original__")
        .map(|f| f.value.as_str())
        .unwrap_or(text);

    ConfessionModData {
        text,
        original_text,
        edit_notes,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChannelErrorKind {
    UnknownChannel,
    MissingPermissions,
    Other,
}

pub fn classify_channel_error(err: &serenity::Error) -> ChannelErrorKind {
    let err_str = err.to_string().to_lowercase();
    if err_str.contains("unknown channel") || err_str.contains("10003") {
        ChannelErrorKind::UnknownChannel
    } else if err_str.contains("invalid permissions")
        || err_str.contains("invalidpermissions")
        || err_str.contains("missing access")
        || err_str.contains("missing permissions")
        || err_str.contains("50001")
        || err_str.contains("50013")
    {
        ChannelErrorKind::MissingPermissions
    } else {
        ChannelErrorKind::Other
    }
}

pub fn build_mod_embed(
    texto: &str,
    estado: &str,
    edit_note: Option<&str>,
    original_text: &str,
) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new()
        .title("📝 Nueva Confesión")
        .description(texto)
        .field(FIELD_STATUS, estado, false)
        .color(if estado.contains("Aprobada") {
            COLOR_APPROVED
        } else if estado.contains("Rechazada") {
            COLOR_REJECTED
        } else {
            COLOR_PENDING
        });

    if texto != original_text {
        embed = embed.field(FIELD_ORIGINAL_TEXT, original_text, false);
    }

    if let Some(note) = edit_note {
        if !note.trim().is_empty() {
            embed = embed.field(FIELD_EDIT_NOTES, note, false);
        }
    }

    embed
}

pub fn build_public_embed(texto: &str, edit_note: Option<&str>) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new()
        .title("Confesión Anónima")
        .description(texto)
        .color(COLOR_PUBLIC)
        .timestamp(serenity::Timestamp::now());

    if let Some(note) = edit_note {
        if !note.trim().is_empty() {
            embed = embed.field("📝 Editado por moderación", note, false);
        }
    }

    embed
}

pub fn format_edit_notes(existing_notes: Option<&str>, new_note: &str) -> Option<String> {
    let trimmed_note = new_note.trim();
    match (existing_notes, trimmed_note.is_empty()) {
        (Some(prev), false) => Some(format!("{}\n• {}", prev, trimmed_note)),
        (None, false) => Some(format!("• {}", trimmed_note)),
        (Some(prev), true) if !prev.trim().is_empty() => Some(prev.to_string()),
        _ => None,
    }
}

pub fn build_action_buttons(enabled: bool) -> serenity::CreateActionRow {
    serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("confession_approve")
            .label("Aprobar")
            .emoji('✅')
            .style(serenity::ButtonStyle::Success)
            .disabled(!enabled),
        serenity::CreateButton::new("confession_edit")
            .label("Editar")
            .emoji('✏')
            .style(serenity::ButtonStyle::Primary)
            .disabled(!enabled),
        serenity::CreateButton::new("confession_reject")
            .label("Rechazar")
            .emoji('❌')
            .style(serenity::ButtonStyle::Danger)
            .disabled(!enabled),
    ])
}

pub async fn has_mod_perms(
    ctx: &serenity::Context,
    guild_id: Option<serenity::GuildId>,
    user_id: serenity::UserId,
) -> bool {
    let Some(guild_id) = guild_id else { return false };

    let Ok(guild) = guild_id.to_partial_guild(ctx).await else {
        return false;
    };

    // Si el usuario es el dueño del servidor, siempre tiene permisos
    if guild.owner_id == user_id {
        return true;
    }

    let Ok(member) = guild_id.member(ctx, user_id).await else {
        return false;
    };

    // Si member.permissions ya tiene valor (por cache), usarlo
    if let Some(permissions) = member.permissions {
        if permissions.contains(serenity::Permissions::MANAGE_MESSAGES)
            || permissions.contains(serenity::Permissions::ADMINISTRATOR)
        {
            return true;
        }
    }

    // Calcular permisos a partir de los roles del miembro en la guild
    for role_id in &member.roles {
        if let Some(role) = guild.roles.get(role_id) {
            if role.permissions.contains(serenity::Permissions::MANAGE_MESSAGES)
                || role.permissions.contains(serenity::Permissions::ADMINISTRATOR)
            {
                return true;
            }
        }
    }

    false
}


/// Envía una confesión anónima al servidor.
#[poise::command(slash_command)]
pub async fn confesion(
    ctx: Context<'_>,
    #[description = "Texto de tu confesión"] texto: String,
) -> Result<(), Error> {
    let author_id = ctx.author().id;

    // Verificar si el usuario es moderador u owner para ignorar el cooldown
    let is_mod_or_owner = has_mod_perms(ctx.serenity_context(), ctx.guild_id(), author_id).await;

    if !is_mod_or_owner {
        let now = std::time::Instant::now();
        let cooldown_duration = std::time::Duration::from_secs(CONFESSION_COOLDOWN_SECS);

        let data = ctx.data();
        let mut cooldowns = data.confession_cooldowns.write().await;

        // Purgar entradas expiradas in-situ para evitar fugas de memoria
        cooldowns.retain(|_, last_time| now.duration_since(*last_time) < cooldown_duration);

        if let Some(&last_time) = cooldowns.get(&author_id) {
            let elapsed = now.duration_since(last_time);
            if elapsed < cooldown_duration {
                let remaining = (cooldown_duration - elapsed).as_secs();
                let msg = if remaining == 1 {
                    "⏳ Debes esperar 1 segundo antes de volver a usar este comando.".to_string()
                } else {
                    format!("⏳ Debes esperar {} segundos antes de volver a usar este comando.", remaining)
                };

                ctx.send(poise::CreateReply::default().content(msg).ephemeral(true)).await?;
                return Ok(());
            }
        }

        cooldowns.insert(author_id, now);
    }

    let data = ctx.data();
    let config_guard = data.confession_config.read().await;

    let Some(config) = config_guard.as_ref() else {
        ctx.send(
            poise::CreateReply::default()
                .content("❌ El sistema de confesiones no está configurado. Un administrador debe ejecutar `/confesiones_setup`.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let mod_channel = config.mod_channel;

    let embed = build_mod_embed(&texto, "⏳ Pendiente", None, &texto);
    let components = vec![build_action_buttons(true)];

    let builder = serenity::CreateMessage::new()
        .embed(embed)
        .components(components);

    if let Err(err) = mod_channel.send_message(ctx.http(), builder).await {
        drop(config_guard);
        match classify_channel_error(&err) {
            ChannelErrorKind::UnknownChannel => {
                data.invalidate_confession_config().await;

                ctx.send(
                    poise::CreateReply::default()
                        .content("❌ Los canales configurados de confesiones ya no existen. Un administrador debe ejecutar `/confesiones_setup` nuevamente.")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }
            ChannelErrorKind::MissingPermissions => {
                ctx.send(
                    poise::CreateReply::default()
                        .content(format!(
                            "❌ No se pudo enviar la confesión: El bot no tiene acceso ni permisos para enviar mensajes en el canal de moderación (<#{}>). Por favor avisa a un administrador del servidor para que agregue al bot en los permisos del canal.",
                            mod_channel
                        ))
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }
            ChannelErrorKind::Other => return Err(err.into()),
        }
    }

    ctx.send(
        poise::CreateReply::default()
            .content("✅ Tu confesión fue enviada. Si es aprobada, aparecerá en el canal de confesiones.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_public_embed_without_notes() {
        // Arrange
        let text = "Esta es una confesión de prueba";

        // Act
        let embed = build_public_embed(text, None);
        let json = serde_json::to_value(&embed).unwrap();

        // Assert
        assert_eq!(json["title"], "Confesión Anónima");
        assert_eq!(json["description"], text);
        assert_eq!(json["color"], 0x5B2C6F);
        assert!(json["fields"].is_null() || json["fields"].as_array().map_or(true, |f| f.is_empty()));
    }

    #[test]
    fn test_build_public_embed_with_notes() {
        // Arrange
        let text = "Texto confesión";
        let note = "• Nota de mod";

        // Act
        let embed = build_public_embed(text, Some(note));
        let json = serde_json::to_value(&embed).unwrap();

        // Assert
        assert_eq!(json["fields"][0]["name"], "📝 Editado por moderación");
        assert_eq!(json["fields"][0]["value"], note);
    }

    #[test]
    fn test_build_public_embed_with_whitespace_note_ignores_note() {
        let text = "Texto confesión";
        let embed = build_public_embed(text, Some("   "));
        let json = serde_json::to_value(&embed).unwrap();
        assert!(json["fields"].is_null() || json["fields"].as_array().map_or(true, |f| f.is_empty()));
    }

    #[test]
    fn test_build_mod_embed_pending() {
        // Arrange
        let text = "Confesión pendiente";

        // Act
        let embed = build_mod_embed(text, "⏳ Pendiente", None, text);
        let json = serde_json::to_value(&embed).unwrap();

        // Assert
        assert_eq!(json["title"], "📝 Nueva Confesión");
        assert_eq!(json["color"], 0xF1C40F); // Amarillo
        assert_eq!(json["fields"][0]["name"], "Estado");
        assert_eq!(json["fields"][0]["value"], "⏳ Pendiente");
    }

    #[test]
    fn test_build_mod_embed_approved_with_diff() {
        // Arrange
        let text = "Texto editado";
        let original = "Texto original";
        let note = "• Se cambió una palabra";

        // Act
        let embed = build_mod_embed(text, "✅ Aprobada por @Mod", Some(note), original);
        let json = serde_json::to_value(&embed).unwrap();

        // Assert
        assert_eq!(json["color"], 0x2ECC71); // Verde
        assert_eq!(json["fields"].as_array().unwrap().len(), 3);
        assert_eq!(json["fields"][0]["value"], "✅ Aprobada por @Mod");
        assert_eq!(json["fields"][1]["name"], "Texto original");
        assert_eq!(json["fields"][1]["value"], original);
        assert_eq!(json["fields"][2]["name"], "Notas de edición");
        assert_eq!(json["fields"][2]["value"], note);
    }

    #[test]
    fn test_build_mod_embed_rejected() {
        // Arrange
        let text = "Confesión rechazada";

        // Act
        let embed = build_mod_embed(text, "❌ Rechazada por @Mod", None, text);
        let json = serde_json::to_value(&embed).unwrap();

        // Assert
        assert_eq!(json["color"], 0xE74C3C); // Rojo
    }

    #[test]
    fn test_build_action_buttons_state() {
        // Arrange & Act
        let row_enabled = build_action_buttons(true);
        let json_enabled = serde_json::to_value(&row_enabled).unwrap();

        let row_disabled = build_action_buttons(false);
        let json_disabled = serde_json::to_value(&row_disabled).unwrap();

        // Assert
        assert_eq!(json_enabled["components"].as_array().unwrap().len(), 3);
        assert_eq!(json_enabled["components"][0]["disabled"], false);

        assert_eq!(json_disabled["components"].as_array().unwrap().len(), 3);
        assert_eq!(json_disabled["components"][0]["disabled"], true);
    }


    #[test]
    fn test_format_edit_notes() {
        // AAA: First note
        let note1 = format_edit_notes(None, "Se removió insulto");
        assert_eq!(note1, Some("• Se removió insulto".to_string()));

        // AAA: Append second note
        let note2 = format_edit_notes(note1.as_deref(), "Se corrigió ortografía");
        assert_eq!(note2, Some("• Se removió insulto\n• Se corrigió ortografía".to_string()));

        // AAA: Empty update preserves existing
        let note3 = format_edit_notes(note2.as_deref(), "   ");
        assert_eq!(note3, note2);

        // AAA: Empty note with no existing
        let note4 = format_edit_notes(None, "");
        assert_eq!(note4, None);
    }

    #[test]
    fn test_extract_mod_embed_data() {
        // Arrange
        let text = "Texto confesión";
        let original = "Texto original confesión";
        let note = "• Nota de mod";
        let create_embed = build_mod_embed(text, "⏳ Pendiente", Some(note), original);
        let json = serde_json::to_value(&create_embed).unwrap();
        let embed: serenity::Embed = serde_json::from_value(json).unwrap();

        // Act
        let extracted = extract_mod_embed_data(&embed);

        // Assert
        assert_eq!(extracted.text, text);
        assert_eq!(extracted.original_text, original);
        assert_eq!(extracted.edit_notes, Some(note));
    }

    #[test]
    fn test_classify_channel_error() {
        // Arrange
        let err_unknown = serenity::Error::Other("Error 10003: Unknown Channel");
        let err_missing = serenity::Error::Other("DiscordJsonError { code: 50001, message: \"Missing Access\" }");
        let err_other = serenity::Error::Other("Timeout connecting to server");

        // Act & Assert
        assert_eq!(classify_channel_error(&err_unknown), ChannelErrorKind::UnknownChannel);
        assert_eq!(classify_channel_error(&err_missing), ChannelErrorKind::MissingPermissions);
        assert_eq!(classify_channel_error(&err_other), ChannelErrorKind::Other);
    }
}



