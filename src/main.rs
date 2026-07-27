mod commands;

use poise::serenity_prelude as serenity;

pub struct Data {}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

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
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                let guild = serenity::GuildId::new(guild_id);
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    guild,
                ).await?;
                tracing::info!("¡Bot listo! Comandos registrados en el servidor {}", guild_id);
                Ok(Data {})
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Error al crear el cliente de Discord");

    client.start().await.expect("Error en la ejecución del cliente");
}
