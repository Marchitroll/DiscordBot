use crate::commands::confession::{
    build_action_buttons, build_mod_embed, build_public_embed, classify_channel_error,
    extract_mod_embed_data, has_mod_perms, ChannelErrorKind,
};
use crate::{Data, Error};
use poise::serenity_prelude as serenity;

pub async fn handle_interaction(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    let serenity::FullEvent::InteractionCreate { interaction } = event else {
        return Ok(());
    };

    match interaction {
        serenity::Interaction::Component(component) => {
            let custom_id = &component.data.custom_id;
            if custom_id.starts_with("ttt_") {
                return crate::commands::tictactoe::handle_move(ctx, component, data).await;
            }
            if !custom_id.starts_with("confession_") {
                return Ok(());
            }

            // Verificar permisos de moderador
            if !has_mod_perms(ctx, component.guild_id, component.user.id).await {
                component
                    .create_response(
                        ctx,
                        serenity::CreateInteractionResponse::Message(
                            serenity::CreateInteractionResponseMessage::new()
                                .content("❌ No tienes permisos para moderar confesiones.")
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                return Ok(());
            }

            match custom_id.as_str() {
                "confession_approve" => {
                    let public_channel = {
                        let config_guard = data.confession_config.read().await;
                        let Some(config) = config_guard.as_ref() else {
                            return Ok(());
                        };
                        config.public_channel
                    };

                    let message = &component.message;
                    let mod_data = message.embeds.first().map(extract_mod_embed_data);
                    let (text, original, edit_note) = match mod_data {
                        Some(ref d) => (d.text, d.original_text, d.edit_notes),
                        None => ("", "", None),
                    };

                    // Publicar en canal público
                    let public_embed = build_public_embed(text, edit_note);
                    if let Err(err) = public_channel
                        .send_message(ctx, serenity::CreateMessage::new().embed(public_embed))
                        .await
                    {
                        match classify_channel_error(&err) {
                            ChannelErrorKind::UnknownChannel => {
                                data.invalidate_confession_config().await;

                                component
                                    .create_response(
                                        ctx,
                                        serenity::CreateInteractionResponse::Message(
                                            serenity::CreateInteractionResponseMessage::new()
                                                .content("❌ El canal público de confesiones ya no existe. Un administrador debe ejecutar `/confesiones-setup` nuevamente.")
                                                .ephemeral(true),
                                        ),
                                    )
                                    .await?;
                                return Ok(());
                            }
                            ChannelErrorKind::MissingPermissions => {
                                component
                                    .create_response(
                                        ctx,
                                        serenity::CreateInteractionResponse::Message(
                                            serenity::CreateInteractionResponseMessage::new()
                                                .content(format!(
                                                    "❌ No se pudo publicar en <#{}>: Al bot le falta el permiso de 'Enviar mensajes' en el canal público. Por favor permite al bot enviar mensajes e insertar enlaces en los ajustes de <#{}>.",
                                                    public_channel, public_channel
                                                ))
                                                .ephemeral(true),
                                        ),
                                    )
                                    .await?;
                                return Ok(());
                            }
                            ChannelErrorKind::Other => return Err(err.into()),
                        }
                    }

                    // Actualizar mensaje de moderación
                    let mod_status = format!("✅ Aprobada por <@{}>", component.user.id);
                    let mod_embed = build_mod_embed(text, &mod_status, edit_note, original);
                    let disabled_buttons = vec![build_action_buttons(false)];

                    component
                        .create_response(
                            ctx,
                            serenity::CreateInteractionResponse::UpdateMessage(
                                serenity::CreateInteractionResponseMessage::new()
                                    .embed(mod_embed)
                                    .components(disabled_buttons),
                            ),
                        )
                        .await?;
                }
                "confession_edit" => {
                    let message = &component.message;
                    let mod_data = message.embeds.first().map(extract_mod_embed_data);
                    let current_text = mod_data.as_ref().map_or("", |d| d.text);

                    let modal = serenity::CreateModal::new("confession_edit_modal", "Editar Confesión")
                        .components(vec![
                            serenity::CreateActionRow::InputText(
                                serenity::CreateInputText::new(
                                    serenity::InputTextStyle::Paragraph,
                                    "Texto Editado",
                                    "edited_text",
                                )
                                .value(current_text)
                                .required(true),
                            ),
                            serenity::CreateActionRow::InputText(
                                serenity::CreateInputText::new(
                                    serenity::InputTextStyle::Short,
                                    "Descripción de cambios",
                                    "edit_description",
                                )
                                .placeholder("Ej: Palabrota cambiada por término suave")
                                .required(false),
                            ),
                        ]);

                    component
                        .create_response(ctx, serenity::CreateInteractionResponse::Modal(modal))
                        .await?;
                }
                "confession_reject" => {
                    let message = &component.message;
                    let mod_data = message.embeds.first().map(extract_mod_embed_data);
                    let (text, original, edit_note) = match mod_data {
                        Some(ref d) => (d.text, d.original_text, d.edit_notes),
                        None => ("", "", None),
                    };

                    let mod_status = format!("❌ Rechazada por <@{}>", component.user.id);
                    let mod_embed = build_mod_embed(text, &mod_status, edit_note, original);
                    let disabled_buttons = vec![build_action_buttons(false)];

                    component
                        .create_response(
                            ctx,
                            serenity::CreateInteractionResponse::UpdateMessage(
                                serenity::CreateInteractionResponseMessage::new()
                                    .embed(mod_embed)
                                    .components(disabled_buttons),
                            ),
                        )
                        .await?;
                }
                _ => {}
            }
        }
        serenity::Interaction::Modal(modal) => {
            if modal.data.custom_id == "confession_edit_modal" {
                let message = match &modal.message {
                    Some(m) => m,
                    None => return Ok(()),
                };

                let mut new_text = String::new();
                let mut edit_desc = String::new();

                for row in &modal.data.components {
                    for component in &row.components {
                        if let serenity::ActionRowComponent::InputText(input) = component {
                            if input.custom_id == "edited_text" {
                                if let Some(val) = &input.value {
                                    new_text = val.clone();
                                }
                            } else if input.custom_id == "edit_description" {
                                if let Some(val) = &input.value {
                                    edit_desc = val.clone();
                                }
                            }
                        }
                    }
                }

                let mod_data = message.embeds.first().map(extract_mod_embed_data);
                let original = mod_data.as_ref().map_or(&new_text[..], |d| d.original_text);
                let existing_notes = mod_data.as_ref().and_then(|d| d.edit_notes);

                let updated_notes = crate::commands::confession::format_edit_notes(existing_notes, &edit_desc);

                let mod_status = format!("✏️ Editada por <@{}> (Pendiente)", modal.user.id);
                let mod_embed = build_mod_embed(&new_text, &mod_status, updated_notes.as_deref(), original);
                let enabled_buttons = vec![build_action_buttons(true)];

                modal
                    .create_response(
                        ctx,
                        serenity::CreateInteractionResponse::UpdateMessage(
                            serenity::CreateInteractionResponseMessage::new()
                                .embed(mod_embed)
                                .components(enabled_buttons),
                        ),
                    )
                    .await?;
            }
        }
        _ => {}
    }

    Ok(())
}
