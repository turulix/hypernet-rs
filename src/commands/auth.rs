use crate::context::{Context, Error};
use poise::CreateReply;
use serenity::all::CreateEmbed;

#[poise::command(slash_command)]
pub async fn auth(ctx: Context<'_>) -> Result<(), Error> {
    let esi = ctx.data().esi.clone();
    let auth_url = esi.get_authorize_url()?;
    let auth_state = auth_url.state;

    let user_id = ctx.author().id.get() as i64;

    let reply = CreateReply::default().ephemeral(true).embed(
        CreateEmbed::new()
            .title("Authorization")
            .description(format!(
                "Click [here]({}-{}) to authorize this bot to access your character information",
                auth_url.authorization_url, user_id
            )),
    );

    sqlx::query_file!(
        "./sql/auth_requests/insert_auth_requests.sql",
        user_id,
        auth_state
    )
    .execute(&ctx.data().postgres)
    .await?;

    ctx.send(reply).await?;

    Ok(())
}
