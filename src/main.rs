use std::hash::{DefaultHasher, Hash, Hasher};

use ::serenity::all::{GuildId, Timestamp};
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
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let guild_id = std::env::var("GUILD_ID")
        .expect("missing GUILD_ID")
        .parse::<GuildId>()
        .expect("GUILD_ID must be a valid u64");
    let intents = serenity::GatewayIntents::all();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![hw_help(), chatgpt(), ticket(), close_ticket(), mute()],
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

///Create a ticket to report something
#[poise::command(slash_command)]
async fn ticket(ctx: Context<'_>) -> Result<(), Error> {
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
async fn close_ticket(ctx: Context<'_>) -> Result<(), Error> {
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

    // We don't need to send a confirmation message because the channel is deleted

    Ok(())
}

/// Mute a user
#[poise::command(slash_command, required_permissions = "VIEW_AUDIT_LOG", prefix_command)]
async fn mute(
    ctx: Context<'_>,
    #[description = "User to mute"] user: serenity::model::user::User,
) -> Result<(), Error> {
    // Get the guild member from the user
    let guild_id = ctx.guild_id().unwrap();
    let mut member = guild_id.member(ctx.http(), user.id).await?;

    // Apply the server mute (this affects voice channels)
    match member
        .disable_communication_until_datetime(
            ctx.http(),
            Timestamp::from_unix_timestamp(1740781600).expect("Error while parsing"),
        )
        .await
    {
        Ok(_) => {
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("Successfully muted {}", user.name))
                    .ephemeral(false),
            )
            .await?;
        }
        Err(e) => {
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("Error while muting {}. Error: {e}", user.name))
                    .ephemeral(false),
            )
            .await?;
        }
    }

    Ok(())
}

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
    println!("{message}");
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
    res
}
