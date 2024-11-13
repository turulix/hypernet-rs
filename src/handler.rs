use crate::context::{AppContext, Error};
use crate::database::eve_character_info::EvECharacterInfo;
use crate::database::hypernet_raffle_model::{
    EvEHypernetRaffle, HypernetRaffleResult, HypernetRaffleStatus,
};
use log::info;
use serenity::all::ComponentType::Button;
use serenity::all::{
    ActivityData, ButtonStyle, Color, ComponentInteractionDataKind, Context, CreateActionRow,
    CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage, Embed, EventHandler, FullEvent, Interaction, InteractionType,
    Ready,
};

pub async fn event_handler(
    ctx: &Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, AppContext, Error>,
    data: &AppContext,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot } => {
            info!("{} is connected!", data_about_bot.user.name);

            // Set the bot's activity
            ctx.set_activity(Some(ActivityData::playing("EvE Online")));
        }
        FullEvent::InteractionCreate { interaction } => {
            if let InteractionType::Component = interaction.kind() {
                let interaction = interaction.as_message_component().unwrap();
                if let ComponentInteractionDataKind::Button = interaction.data.kind {
                    if interaction.data.custom_id.starts_with("raffle-won:")
                        || interaction.data.custom_id.starts_with("raffle-lost:")
                    {
                        // Handle the win of a raffle
                        let raffle_id =
                            interaction.data.custom_id["raffle-won:".len()..].to_string();

                        let raffle: EvEHypernetRaffle = sqlx::query_file_as!(
                            EvEHypernetRaffle,
                            "./sql/hypernet_raffle/select_raffle_by_id.sql",
                            raffle_id
                        )
                        .fetch_one(&data.postgres)
                        .await?;

                        let character_info: EvECharacterInfo = sqlx::query_file_as!(
                            EvECharacterInfo,
                            "./sql/eve_character/select_character_by_id.sql",
                            raffle.character_id
                        )
                        .fetch_one(&data.postgres)
                        .await?;

                        if interaction.user.id != character_info.discord_user_id as u64 {
                            interaction
                                .create_response(
                                    &ctx,
                                    CreateInteractionResponse::Message(
                                        CreateInteractionResponseMessage::new()
                                            .ephemeral(true)
                                            .content(
                                                "This is not your raffle.\n\
                                                You can't interact with this embed.\n\
                                                If this is your character, please use `/auth` to link your discord account to your character."
                                            )
                                    ),
                                )
                                .await?;
                            return Ok(());
                        }

                        if interaction.data.custom_id.starts_with("raffle-won:") {
                            sqlx::query_file!(
                                "./sql/hypernet_raffle/update_result.sql",
                                raffle_id,
                                HypernetRaffleResult::Winner as HypernetRaffleResult,
                            )
                            .execute(&data.postgres)
                            .await?;

                            let edited_embeds: Vec<CreateEmbed> = interaction
                                .message
                                .embeds
                                .iter()
                                .map(|embed| {
                                    CreateEmbed::from(embed.clone())
                                        .title(format!(
                                            "{} - Won",
                                            embed.title.clone().unwrap_or("".to_string())
                                        ))
                                        .color(Color::from_rgb(241, 196, 15))
                                })
                                .collect();

                            interaction
                                .create_response(
                                    &ctx,
                                    CreateInteractionResponse::UpdateMessage(
                                        CreateInteractionResponseMessage::new()
                                            .embeds(edited_embeds)
                                            .components(vec![CreateActionRow::Buttons(
                                                create_disabled_raffle_buttons(&raffle_id),
                                            )]),
                                    ),
                                )
                                .await?;
                        } else {
                            sqlx::query_file!(
                                "./sql/hypernet_raffle/update_result.sql",
                                raffle_id,
                                HypernetRaffleResult::Loser as HypernetRaffleResult,
                            )
                            .execute(&data.postgres)
                            .await?;

                            let edited_embeds: Vec<CreateEmbed> = interaction
                                .message
                                .embeds
                                .iter()
                                .map(|embed| {
                                    CreateEmbed::from(embed.clone())
                                        .title(format!(
                                            "{} - Loss",
                                            embed.title.clone().unwrap_or("".to_string())
                                        ))
                                        .color(Color::from_rgb(230, 126, 34))
                                })
                                .collect();

                            interaction
                                .create_response(
                                    &ctx,
                                    CreateInteractionResponse::UpdateMessage(
                                        CreateInteractionResponseMessage::new()
                                            .embeds(edited_embeds)
                                            .components(vec![CreateActionRow::Buttons(
                                                create_disabled_raffle_buttons(&raffle_id),
                                            )]),
                                    ),
                                )
                                .await?;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn create_disabled_raffle_buttons(raffle_id: &str) -> Vec<CreateButton> {
    vec![
        CreateButton::new("raffle-won:".to_string() + raffle_id)
            .label("Won Raffle")
            .style(ButtonStyle::Success)
            .disabled(true),
        CreateButton::new("raffle-lost:".to_string() + raffle_id)
            .label("Lost Raffle")
            .style(ButtonStyle::Danger)
            .disabled(true),
    ]
}
