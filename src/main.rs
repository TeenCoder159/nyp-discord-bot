use ::serenity::all::GuildId;
use chrono::{DateTime, Local};
// use mistralai_client::v1::{
//     chat::{ChatMessage, ChatMessageRole, ChatParams},
//     // client::Client,
//     constants::Model,
// };
use poise::serenity_prelude as serenity;
use std::sync::LazyLock;

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
            commands: vec![help(), mistral()],
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

static LAST_HELP_CALL: LazyLock<DateTime<Local>> = LazyLock::new(Local::now);

/// Ping the helpers
#[poise::command(
    slash_command,
    required_permissions = "SEND_MESSAGES",
    on_error = "error_handler"
)]
async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let response: &str;

    let current_time = Local::now();
    let time_diff = (*LAST_HELP_CALL - current_time).num_minutes();
    if time_diff > 1 {
        response = "<@&1344212981038317578>"
    } else {
        response = "Too early!\nPlease wait at least 15 minutes after asking for help.";
    }

    ctx.say(response).await?;
    Ok(())
}
#[poise::command(slash_command, prefix_command)]
async fn mistral(ctx: Context<'_>, #[description = "Input"] input: String) -> Result<(), Error> {
    // let user_input = input;
    // let client = Client::new(None, None, None, None).unwrap();

    // let model = Model::OpenMistral7b;
    // let messages = vec![ChatMessage {
    //     role: ChatMessageRole::User,
    //     content: user_input.clone(),
    //     tool_calls: None,
    // }];
    // let options = ChatParams {
    //     temperature: 0.0,
    //     random_seed: Some(42),
    //     ..Default::default()
    // };
    // let result = client.chat(model, messages, Some(options)).unwrap();
    // let response = &result.choices[0].message.content;

    // let client = Client::new(None, None, None, None).unwrap();

    // let model = Model::OpenMistral7b;
    // let messages = vec![ChatMessage {
    //     role: ChatMessageRole::User,
    //     content: input,
    //     tool_calls: None,
    // }];
    // let options = ChatParams {
    //     temperature: 0.0,
    //     random_seed: Some(42),
    //     ..Default::default()
    // };

    // let result = client
    //     .chat_async(model, messages, Some(options))
    //     .await
    //     .unwrap();
    // let response = &result.choices[0].message.content;
    let response = " ";

    ctx.say(response).await?;
    Ok(())
}

async fn error_handler(error: poise::FrameworkError<'_, Data, Error>) {
    println!("Oh no, we got an error: {:?}", error);
}
