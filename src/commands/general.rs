use crate::{Context, Error};
use std::time::{SystemTime, UNIX_EPOCH};

struct Provider {
    name: &'static str,
    base_url: &'static str,
}

const PROVIDERS: &[Provider] = &[
    Provider { name: "HTTP Cats", base_url: "https://http.cat" },
    Provider { name: "HTTP Dogs", base_url: "https://http.dog" },
    Provider { name: "HTTP Ducks", base_url: "https://httpducks.com" },
    Provider { name: "HTTP Goats", base_url: "https://httpgoats.com" },
    Provider { name: "HTTP Pizza", base_url: "https://http.pizza" },
];

/// Responde para verificar si el bot esta activo.
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("🏓 ¡Pong!").await?;
    Ok(())
}

/// Repite el texto ingresado.
#[poise::command(slash_command)]
pub async fn echo(
    ctx: Context<'_>,
    #[description = "El mensaje a repetir"] message: String,
) -> Result<(), Error> {
    ctx.say(message).await?;
    Ok(())
}

/// Muestra una imagen asociada a un codigo de estado HTTP.
#[poise::command(slash_command)]
pub async fn http(
    ctx: Context<'_>,
    #[description = "Codigo de estado HTTP (ej. 404, 200, 500)"] code: u16,
) -> Result<(), Error> {
    let status_code = match poise::serenity_prelude::http::StatusCode::from_u16(code) {
        Ok(sc) => sc,
        Err(_) => {
            ctx.say(format!("❌ `{}` no es un código de estado HTTP reconocido.", code)).await?;
            return Ok(());
        }
    };

    let status_title = status_code.canonical_reason().unwrap_or("Estado Desconocido");

    // Seleccion aleatoria de proveedor basada en nanosegundos.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);

    let provider = &PROVIDERS[(nanos as usize) % PROVIDERS.len()];
    let image_url = format!("{}/{}.jpg", provider.base_url, code);

    // Color del embed segun el grupo del codigo HTTP.
    let color = match code / 100 {
        2 => 0x2ECC71, // 2xx Exito
        3 => 0x3498DB, // 3xx Redireccion
        4 => 0xF1C40F, // 4xx Error del cliente
        5 => 0xE74C3C, // 5xx Error del servidor
        _ => 0x95A5A6,
    };

    ctx.send(poise::CreateReply::default().embed(
        poise::serenity_prelude::CreateEmbed::new()
            .title(format!("HTTP {} - {}", code, status_title))
            .image(image_url)
            .color(color)
            .footer(poise::serenity_prelude::CreateEmbedFooter::new(format!(
                "Fuente: {}",
                provider.name
            ))),
    )).await?;

    Ok(())
}
