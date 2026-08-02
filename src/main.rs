mod commands;

use poise::serenity_prelude as serenity;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct Data {
    pub confession_config: RwLock<Option<commands::confession::ConfessionConfig>>,
    pub confession_cooldowns: RwLock<std::collections::HashMap<serenity::UserId, std::time::Instant>>,
}

impl Data {
    pub async fn invalidate_confession_config(&self) {
        {
            let mut config_write = self.confession_config.write().await;
            *config_write = None;
        }
        commands::setup::delete_config_file();
    }
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::CooldownHit {
            remaining_cooldown,
            ctx,
            ..
        } => {
            let secs = remaining_cooldown.as_secs();
            let msg = if secs == 1 {
                "⏳ Debes esperar 1 segundo antes de volver a usar este comando.".to_string()
            } else {
                format!("⏳ Debes esperar {} segundos antes de volver a usar este comando.", secs)
            };
            let _ = ctx
                .send(poise::CreateReply::default().content(msg).ephemeral(true))
                .await;
        }
        poise::FrameworkError::MissingUserPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            let msg = format!(
                "❌ No tienes los permisos necesarios para usar este comando (requiere: {:?}).",
                missing_permissions
            );
            let _ = ctx
                .send(poise::CreateReply::default().content(msg).ephemeral(true))
                .await;
        }
        poise::FrameworkError::MissingBotPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            tracing::error!(
                "Faltan permisos en el bot para ejecutar el comando '{}': {:?}",
                ctx.command().name,
                missing_permissions
            );
            let msg = format!(
                "❌ El bot no tiene los permisos necesarios en el servidor para ejecutar este comando (faltan: {:?}).",
                missing_permissions
            );
            let _ = ctx
                .send(poise::CreateReply::default().content(msg).ephemeral(true))
                .await;
        }
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!("Error ejecutando comando '{}': {:?}", ctx.command().name, error);
            let _ = ctx
                .send(
                    poise::CreateReply::default()
                        .content(format!("❌ Ocurrió un error al ejecutar el comando: {}", error))
                        .ephemeral(true),
                )
                .await;
        }
        other => {
            tracing::error!("Error en la ejecución del framework: {:?}", other);
            if let Err(e) = poise::builtins::on_error(other).await {
                tracing::error!("Error en el manejador por defecto de Poise: {:?}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let token = std::env::var("DISCORD_TOKEN")
        .expect("DISCORD_TOKEN debe estar configurado en el archivo .env");

    let guild_id: u64 = std::env::var("GUILD_ID")
        .expect("GUILD_ID debe estar configurado en el archivo .env")
        .parse()
        .expect("GUILD_ID debe ser un número entero u64 válido");

    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            on_error: |error| Box::pin(on_error(error)),
            event_handler: |ctx, event, framework, data| {
                Box::pin(async move {
                    commands::interactions::handle_interaction(ctx, event, framework, data).await
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                let guild = serenity::GuildId::new(guild_id);
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    guild,
                )
                .await?;

                // Cargar configuración guardada si existe
                let loaded_config = commands::setup::load_config_file().map(|(mod_id, pub_id)| {
                    commands::confession::ConfessionConfig {
                        mod_channel: serenity::ChannelId::new(mod_id),
                        public_channel: serenity::ChannelId::new(pub_id),
                    }
                });

                if let Some(ref config) = loaded_config {
                    tracing::info!(
                        "Configuración de confesiones cargada desde archivo (Mod: {}, Pública: {})",
                        config.mod_channel,
                        config.public_channel
                    );
                } else {
                    tracing::info!("No se encontró configuración previa de confesiones.");
                }

                tracing::info!("¡Bot listo! Comandos registrados en el servidor {}", guild_id);
                Ok(Data {
                    confession_config: RwLock::new(loaded_config),
                    confession_cooldowns: RwLock::new(std::collections::HashMap::new()),
                })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Error al crear el cliente de Discord");

    client.start().await.expect("Error en la ejecución del cliente");
}
