use ::serenity::all::GuildId;
use poise::serenity_prelude as serenity;
use serde_json::Value;

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
            commands: vec![help(), chatgpt()],
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
#[poise::command(slash_command)]
async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let response: String;
    {
        let mut cooldown_tracker = ctx.command().cooldowns.lock().unwrap();

        let mut cooldown_durations = poise::CooldownConfig::default();

        cooldown_durations.guild = Some(std::time::Duration::from_secs(15 * 60));

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

//

#[poise::command(slash_command, prefix_command)]
async fn chatgpt(
    ctx: Context<'_>,
    #[description = "Input to ChatGPT"] input: String,
) -> Result<(), Error> {
    // Defer the response once to indicate the bot is processing
    ctx.defer().await?;

    // Send a temporary message
    ctx.say("Generating response, please wait...").await?;

    // Get response from API
    let output = get(input).await;

    // Parse the response
    let message: String = match output.lines().find(|x| x.contains("content\"")) {
        Some(line) => {
            let (_, a) = line
                .split_once("content\":\"")
                .expect("Failed to parse content");
            let (b, _) = a.split_once("}").expect("Failed to parse content end");
            b.to_string()
        }
        None => "Sorry, I couldn't generate a response.".to_string(),
    };

    // Send the actual response
    ctx.say(message).await?;

    Ok(())
}

async fn get(input: String) -> String {
    let client = reqwest::Client::new();
    let v: Value = serde_json::from_str(
        std::fs::read_to_string("input.json")
            .expect("Error while reading")
            .replace(
                r#""Who is the best French painter? Answer in one short sentence.""#,
                format!("\"{}\"", input).as_str(),
            )
            .as_str(),
    )
    .expect("GO TO HELL");

    let res = client
        .post("https://api.mistral.ai/v1/chat/completions")
        .bearer_auth(std::env::var("MISTRAL_API_KEY").expect("Error getting API KEY"))
        .json(&v)
        .send()
        .await
        .expect("msg")
        .text()
        .await
        .expect("msg");
    println!("{res}");
    res
}
