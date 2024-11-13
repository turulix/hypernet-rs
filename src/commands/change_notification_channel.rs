use crate::context::{Context, Error};
use poise::CreateReply;
use serenity::all::{ChannelId, CreateEmbed, Mentionable};

#[poise::command(slash_command)]
pub async fn change_notification_channel(
    ctx: Context<'_>,
    #[channel_types("Text")] channel: Option<ChannelId>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get() as i64;
    let channel_id = channel.map(|c| c.get() as i64);
    sqlx::query_file!("./sql/notification_channel/update_notification_channel.sql", user_id, channel_id)
        .execute(&ctx.data().postgres)
        .await?;

    let selected_channel = match channel {
        Some(c) => c.mention().to_string(),
        None => "Private messages".to_string(),
    };

    let reply = CreateReply::default().ephemeral(true).embed(
        CreateEmbed::new()
            .title("Notification Channel Changed.")
            .description(format!("Notification channel set to {}", selected_channel)),
    );

    ctx.send(reply).await?;
    Ok(())
}
