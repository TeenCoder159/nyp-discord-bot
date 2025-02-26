use ::serenity::all::GuildId;
use poise::serenity_prelude as serenity;

#[derive(Debug)]
struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let guild_id = std::env::var("GUILD_ID")
        .expect("missing GUILD_ID")
        .parse::<GuildId>()
        .expect("GUILD_ID must be a valid u64");
    let intents = serenity::GatewayIntents::all();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![help()],
            manual_cooldowns: true,
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                // Register commands in the specified guild
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id)
                    .await?;
                Ok(Data {})
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .expect("Error creating client");

    client.start().await.expect("Error starting client");
}

/// Ping the helpers
#[poise::command(slash_command, required_bot_permissions = "ADMINISTRATOR")]
async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let response: String;
    {
        let mut cooldown_tracker = ctx.command().cooldowns.lock().unwrap();

        let mut cooldown_durations = poise::CooldownConfig::default();
        cooldown_durations.user = Some(std::time::Duration::from_secs(15 * 60));

        match cooldown_tracker.remaining_cooldown(ctx.cooldown_context(), &cooldown_durations) {
            Some(remaining) => {
                let minutes = remaining.as_secs() / 60;
                let seconds = remaining.as_secs() % 60;

                response = format!("Please wait {}m {}s", minutes, seconds);
            }
            None => {
                cooldown_tracker.start_cooldown(ctx.cooldown_context());
                response = "<@&1344212981038317578>".to_string();
            }
        }
    }
    use serenity::builder::CreateAllowedMentions as Am;
    ctx.send(
        poise::CreateReply::default()
            .content(&response)
            .allowed_mentions(Am::new().roles(vec![1344212981038317578])),
    )
    .await?;
    Ok(())
}
