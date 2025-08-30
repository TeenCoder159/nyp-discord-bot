use std::hash::{DefaultHasher, Hash, Hasher};
// use std::thread;

use dotenv::dotenv;
use ::serenity::all::{ChannelId, CreateMessage, GuildId};
use poise::serenity_prelude as serenity;
use serde_json::Value;
use serenity::builder::CreateChannel;
use serenity::model::channel::ChannelType;
use serenity::model::channel::{PermissionOverwrite, PermissionOverwriteType};
use serenity::model::permissions::Permissions;

#[derive(Debug)]
struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let guild_id = std::env::var("GUILD_ID")
        .expect("missing GUILD_ID")
        .parse::<GuildId>()
        .expect("GUILD_ID must be a valid u64");
    let intents = serenity::GatewayIntents::all();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                hw_help(),
                chatgpt(),
                ticket(),
                close_ticket(),
                bored(),
                links(),
            ],
            event_handler: |ctx, event, framework, data| {
                Box::pin(async move { event_handler(ctx, event, framework, data).await })
            },
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
async fn hw_help(ctx: Context<'_>) -> Result<(), Error> {
    println!("Homework help command called by: {}", ctx.author());
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

    log(&format!("Homework help called by: {}", ctx.author()), &ctx)
        .await
        .expect("Error while logging homework help");

    use serenity::builder::CreateAllowedMentions as Am;
    ctx.send(
        poise::CreateReply::default()
            .content(&response)
            .allowed_mentions(Am::new().roles(vec![1344212981038317578])),
    )
    .await?;
    Ok(())
}

/// Create a ticket to report something
#[poise::command(slash_command)]
async fn ticket(ctx: Context<'_>) -> Result<(), Error> {
    println!("Ticket command called by: {}", ctx.author());
    // Get the user's ID and extract the first 4 digits
    let user_id = ctx.author().id.to_string();
    let mut hashed_prefix = DefaultHasher::new();
    user_id.hash(&mut hashed_prefix);

    let id_prefix = if user_id.len() >= 4 {
        &hashed_prefix.finish().to_string()[0..4]
    } else {
        &hashed_prefix.finish().to_string()
    };

    // Get guild information
    let guild = ctx.guild_id().unwrap();

    // Create permission overwrites array
    let mut perms = Vec::new();

    // Default deny permission for everyone (making channel private)
    perms.push(PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
        kind: PermissionOverwriteType::Role(guild.everyone_role()),
    });

    // Allow permission for ticket creator
    perms.push(PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(ctx.author().id),
    });

    // Find and add permissions for all admin roles
    if let Ok(guild_roles) = ctx.guild_id().unwrap().roles(ctx.http()).await {
        for (role_id, role) in guild_roles {
            if role.permissions.contains(Permissions::ADMINISTRATOR) {
                perms.push(PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(role_id),
                });
            }
        }
    }

    log(&format!("CREATE TICKET CALLED BY: {}", ctx.author()), &ctx)
        .await
        .expect("Error logging create ticket");

    // Create the channel with the first 4 digits of the user ID
    let ticket = CreateChannel::new(format!("ticket-{}", id_prefix))
        .kind(ChannelType::Text)
        .permissions(perms);

    // Properly await the channel creation and handle errors
    match guild.create_channel(ctx.http(), ticket).await {
        Ok(channel) => {
            // Send an ephemeral message (only visible to the command invoker)
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("Your ticket has been created: <#{}>", channel.id))
                    .ephemeral(true),
            )
            .await?;
        }
        Err(e) => {
            // Also make error message ephemeral
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("Failed to create ticket: {}", e))
                    .ephemeral(true),
            )
            .await?;
            eprintln!("Error creating ticket channel: {}", e);
        }
    }

    Ok(())
}

#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
/// Close an exisiting ticket (Admin only)
async fn close_ticket(ctx: Context<'_>) -> Result<(), Error> {
    println!("Close ticket command called by: {}", ctx.author());
    // Check if the channel is a ticket channel
    let channel = ctx.channel_id().to_channel(ctx.http()).await?;
    let channel_name = channel.guild().unwrap().name;

    if !channel_name.contains("ticket") {
        ctx.send(
            poise::CreateReply::default()
                .content("This is not a ticket channel.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    // Delete the channel
    ctx.channel_id().delete(ctx.http()).await?;
    log(&format!("CLOSE TICKET CALLED BY {}", ctx.author()), &ctx)
        .await
        .expect("Error while logging close_ticket");
    Ok(())
}

async fn event_handler(
    ctx: &serenity::Context, // Change this line
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        poise::serenity_prelude::FullEvent::GuildMemberAddition { new_member } => {
            let _greet_channel = ChannelId::new(1344976093332901958)
                .send_message(
                    &ctx.http, // Change this line
                    CreateMessage::new().content(format!("Hello {new_member}")),
                )
                .await?;
            println!("Greeted: {}", new_member);
        }
        _ => {}
    }
    Ok(())
}

// #[poise::command(slash_command)]
// async fn bored(
//     ctx: &serenity::Context, // Change this line
//     event: &serenity::FullEvent,
//     _framework: poise::FrameworkContext<'_, Data, Error>,
//     _data: &Data,
// ) -> Result<(), Error> {
// }
#[poise::command(slash_command)]
/// Are you bored?
async fn bored(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Hi Bored, I'm the NYP SIT Bot!").await?;
    println!("Bored command called by: {}", ctx.author());
    log(&format!("Bored command called by: {}", ctx.author()), &ctx)
        .await
        .expect("Error logging bored command");
    Ok(())
}

#[poise::command(slash_command)]
/// Get the useful links for NYP
async fn links(ctx: Context<'_>) -> Result<(), Error> {
    println!("Links command called by: {}", ctx.author());
    log(&format!("Links command called by: {}", ctx.author()), &ctx)
        .await
        .expect("Error logging links");
    let sit_link = std::env::var("TELE").expect("Telegram Link not set");
    let discord_link = std::env::var("DISC").expect("Discord Link not set");
    let nyp_link = std::env::var("NYP").expect("Telegram Link not set");
    let message = format!(
        "\n# Invite links:\nSIT Telegram: {sit_link}\nNYP Telegram: {nyp_link}\nDiscord: {discord_link}\n\n# Useful websites:\nPlease look in the resources channel: <#1344960997437210687>\n"
    );
    ctx.say(message).await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command)]
/// Talk with Mistral, misleading, I know
async fn chatgpt(
    ctx: Context<'_>,
    #[description = "Input to ChatGPT"] input: String,
) -> Result<(), Error> {
    // Defer the response once to indicate the bot is processing
    ctx.defer().await?;

    // Send a temporary message
    ctx.say("Generating response, please wait...").await?;

    // Get response from API
    let output = get(input.clone()).await;

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
    ctx.say(message.clone()).await?;

    println!(
        "Chatgpt called by: {} with prompt: {}\nOutput generated: {}",
        ctx.author(),
        input,
        message
    );

    log(
        &format!(
            "CHATGPT CALLED BY: {} WITH PROMPT: {}, AND OUTPUT: {}",
            ctx.author(),
            input,
            message
        ),
        &ctx,
    )
    .await
    .expect("Error with Logging chatgpt");

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
    res
}

async fn log(input: &str, ctx: &Context<'_>) -> Result<(), Box<Error>> {
    match ChannelId::new(1375117906207309905)
        .send_message(
            ctx.http(),
            CreateMessage::new().content(format!("LOG: {}", input)),
        )
        .await
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{e}")
        }
    };

    Ok(())
}
