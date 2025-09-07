use crate::context::CronAppContext;
use crate::cron::collect_hypernet_task::ProfitType::{Loser, Winner};
use crate::cron::CronTask;
use crate::database::eve_character_info::EvECharacterInfo;
use crate::database::hypernet_raffle_model::{
    EvEHypernetRaffle, HypernetRaffleResult, HypernetRaffleStatus,
};
use anyhow::anyhow;
use async_trait::async_trait;
use log::{debug, warn};
use rfesi::groups::Notification;
use rfesi::prelude::Esi;
use serenity::all::{
    ButtonStyle, ChannelId, Colour, CreateButton, CreateEmbed, CreateEmbedFooter, CreateMessage,
};
use serenity::builder::CreateActionRow;
use sqlx::{query_file, query_file_as, Executor};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;
use thousands::Separable;

pub struct CollectHypernetTask;

#[async_trait]
impl CronTask for CollectHypernetTask {
    fn name(&self) -> &'static str {
        "CollectHypernetTask"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(600)
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn run(&self, ctx: CronAppContext) -> anyhow::Result<()> {
        let all_chars: Vec<EvECharacterInfo> = query_file_as!(
            EvECharacterInfo,
            "./sql/eve_character/select_all_characters.sql"
        )
        .fetch_all(&ctx.postgres)
        .await?;

        // Prices for each type_id, (sell, buy)
        let mut prices: HashMap<i32, (Option<f64>, Option<f64>)> = HashMap::new();
        let hypernet_core_orders = loop {
            let hypernet_core_orders = ctx
                .esi
                .group_market()
                .get_region_orders(10000002, None, None, Some(52568))
                .await;

            if let Ok(orders) = hypernet_core_orders {
                break orders;
            } else {
                warn!("Error fetching orders for Hypercore. Retrying...");
            }
        };

        let hypernet_core_sell_price = hypernet_core_orders
            .iter()
            .filter(|x| !x.is_buy_order)
            .map(|x| x.price)
            .min_by(|a, b| a.partial_cmp(b).unwrap());

        let hypernet_core_buy_price = hypernet_core_orders
            .iter()
            .filter(|x| x.is_buy_order)
            .map(|x| x.price)
            .max_by(|a, b| a.partial_cmp(b).unwrap());

        let plex_price = ctx
            .esi
            .group_market()
            .get_market_prices()
            .await?
            .iter()
            .find(|x| x.type_id == 44992) // PLEX
            .and_then(|x| x.average_price);

        for char in all_chars {
            let res = handle_character(
                &ctx,
                char,
                &mut prices,
                &hypernet_core_sell_price,
                &hypernet_core_buy_price,
                &plex_price,
            )
            .await;
            if let Err(e) = res {
                log::error!("Error handling character: {:?}", e);
            }
        }

        Ok(())
    }
}

async fn handle_character(
    ctx: &CronAppContext,
    char: EvECharacterInfo,
    price_cache: &mut HashMap<i32, (Option<f64>, Option<f64>)>,
    hypernet_core_sell_price: &Option<f64>,
    hypernet_core_buy_price: &Option<f64>,
    plex_price: &Option<f64>,
) -> anyhow::Result<()> {
    let mut esi = ctx.esi.clone();
    let notification_channel_id: Option<i64> = sqlx::query_file_scalar!(
        "./sql/notification_channel/select_channel_for_user.sql",
        char.character_id
    )
    .fetch_optional(&ctx.postgres)
    .await?
    .flatten();

    esi.use_refresh_token(&char.refresh_token).await?;

    let notifications = esi
        .group_character()
        .get_notifications(char.character_id)
        .await?;

    let raffles_created: Vec<_> = notifications
        .iter()
        .filter(|n| n.notification_type == "RaffleCreated")
        .collect();

    let raffles_expired: Vec<_> = notifications
        .iter()
        .filter(|n| n.notification_type == "RaffleExpired")
        .collect();

    let raffles_finished: Vec<_> = notifications
        .iter()
        .filter(|n| n.notification_type == "RaffleFinished")
        .collect();

    let raffles_created = parse_raffles(&raffles_created, char.character_id)?;
    let raffles_expired = parse_raffles(&raffles_expired, char.character_id)?;
    let raffles_finished = parse_raffles(&raffles_finished, char.character_id)?;

    // Insert new raffles
    let mut transaction = ctx.postgres.begin().await?;
    for raffle in raffles_created.iter().cloned() {
        let mut prices = price_cache.get(&raffle.type_id);
        if prices.is_none() {
            let orders = loop {
                let orders = esi
                    .group_market()
                    .get_region_orders(
                        10000002, // The Forge
                        None,
                        None,
                        Some(raffle.type_id),
                    )
                    .await;

                if let Ok(orders) = orders {
                    break orders;
                } else {
                    warn!(
                        "Error fetching orders for type_id: {}. Retrying...",
                        raffle.type_id
                    );
                }
            };

            let sell_orders = orders
                .iter()
                .filter(|x| !x.is_buy_order)
                .collect::<Vec<_>>();
            let buy_orders = orders.iter().filter(|x| x.is_buy_order).collect::<Vec<_>>();

            let sell_price = sell_orders
                .iter()
                .filter(|x| x.location_id == 60003760) // Jita 4-4
                .map(|x| x.price)
                .min_by(|a, b| a.partial_cmp(b).unwrap());

            let buy_price = buy_orders
                .iter()
                .filter(|x| x.location_id == 60003760) // Jita 4-4
                .map(|x| x.price)
                .max_by(|a, b| a.partial_cmp(b).unwrap());

            price_cache.insert(raffle.type_id, (sell_price, buy_price));
            prices = price_cache.get(&raffle.type_id);
        }
        let prices = prices.unwrap_or(&(None, None));

        let query = query_file!(
            "./sql/hypernet_raffle/insert_raffle.sql",
            raffle.location_id,
            raffle.owner_id,
            raffle.character_id,
            raffle.raffle_id,
            raffle.ticket_count,
            raffle.ticket_price,
            raffle.type_id,
            raffle.status as HypernetRaffleStatus,
            raffle.result as HypernetRaffleResult,
            raffle.created_at,
            prices.0,
            prices.1,
            hypernet_core_buy_price.clone(),
            hypernet_core_sell_price.clone(),
            plex_price.clone()
        );
        transaction.execute(query).await?;
    }
    transaction.commit().await?;

    // Figure out which raffles have expired and which have finished
    let created_raffle_ids: HashSet<String> = raffles_created
        .iter()
        .map(|r| r.raffle_id.clone())
        .collect();
    let expired_raffle_ids: HashSet<String> = raffles_expired
        .iter()
        .map(|r| r.raffle_id.clone())
        .collect();
    let finished_raffle_ids: HashSet<String> = raffles_finished
        .iter()
        .map(|r| r.raffle_id.clone())
        .collect();

    // Raffle Ids that are both created and expired/finished
    let newly_expired_raffle_ids: HashSet<String> = created_raffle_ids
        .intersection(&expired_raffle_ids)
        .cloned()
        .collect();
    let newly_finished_raffle_ids: HashSet<String> = created_raffle_ids
        .intersection(&finished_raffle_ids)
        .cloned()
        .collect();

    //TODO: We should fetch the DB raffles to ensure its not already finished.

    // Raffles to check
    let raffles_to_check = newly_expired_raffle_ids
        .union(&newly_finished_raffle_ids)
        .cloned()
        .collect::<Vec<_>>();

    for raffle_id in raffles_to_check {
        let raffle: EvEHypernetRaffle = query_file_as!(
            EvEHypernetRaffle,
            "./sql/hypernet_raffle/select_raffle_by_id.sql",
            raffle_id
        )
        .fetch_one(&ctx.postgres)
        .await?;

        debug!("Checking raffle {}", raffle_id);

        if let HypernetRaffleStatus::Created = raffle.status {
            let (embed, status) = if newly_expired_raffle_ids.contains(&raffle_id) {
                // Send Expired Discord Notification.
                (
                    build_embed(&raffle, &esi, HypernetRaffleStatus::Expired).await?,
                    HypernetRaffleStatus::Expired,
                )
            } else if newly_finished_raffle_ids.contains(&raffle_id) {
                (
                    build_embed(&raffle, &esi, HypernetRaffleStatus::Finished).await?,
                    HypernetRaffleStatus::Finished,
                )
            } else {
                unreachable!();
            };

            let channel_id = ChannelId::new(
                notification_channel_id.ok_or(anyhow!("No notification channel set for user"))?
                    as u64,
            );
            let mut message = CreateMessage::new().embed(embed);

            if let HypernetRaffleStatus::Finished = status {
                let won_button = CreateButton::new(format!("raffle-won:{}", raffle.raffle_id))
                    .style(ButtonStyle::Success)
                    .label("Won Raffle");
                let lost_button = CreateButton::new(format!("raffle-lost:{}", raffle.raffle_id))
                    .style(ButtonStyle::Danger)
                    .label("Lost Raffle");
                let action_row = CreateActionRow::Buttons(vec![won_button, lost_button]);
                message = message.components(vec![action_row]);
            }

            channel_id.send_message(&ctx.discord_http, message).await?;
        }
    }

    // Update raffle statuses
    let mut transaction = ctx.postgres.begin().await?;
    for raffle in raffles_expired {
        let query = query_file!(
            "./sql/hypernet_raffle/update_status.sql",
            raffle.raffle_id,
            HypernetRaffleStatus::Expired as HypernetRaffleStatus,
        );
        transaction.execute(query).await?;
    }

    for raffle in raffles_finished {
        let query = query_file!(
            "./sql/hypernet_raffle/update_status.sql",
            raffle.raffle_id,
            HypernetRaffleStatus::Finished as HypernetRaffleStatus,
        );
        transaction.execute(query).await?;
    }
    transaction.commit().await?;

    Ok(())
}

async fn build_embed(
    raffle: &EvEHypernetRaffle,
    esi: &Esi,
    current_status: HypernetRaffleStatus,
) -> Result<CreateEmbed, anyhow::Error> {
    let item = esi.group_universe().get_type(raffle.type_id).await?;
    let color = match current_status {
        HypernetRaffleStatus::Expired => Colour::from((255, 0, 0)),
        HypernetRaffleStatus::Finished => Colour::from((0, 255, 0)),
        _ => Colour::from((255, 255, 255)),
    };
    let profit_win = calculate_profit(raffle, Winner);
    let profit_lose = calculate_profit(raffle, Loser);

    let expected_value = (|| Some((profit_lose? + profit_win?) * 0.5))();

    Ok(CreateEmbed::new()
        .title(format!("Hypernet Raffle {}", current_status))
        .description(format!(
            "Hypernet Raffle changed status to {}",
            current_status
        ))
        .thumbnail(format!(
            "https://images.evetech.net/types/{}/icon",
            raffle.type_id
        ))
        .color(color)
        .field("Item", item.name, true)
        .field(
            "Marked Value (Sell)",
            raffle
                .sell_price
                .map(|x| x.separate_with_dots())
                .unwrap_or("Unknown".to_string()),
            true,
        )
        .field(
            "Marked Value (Buy)",
            raffle
                .buy_price
                .map(|x| x.separate_with_dots())
                .unwrap_or("Unknown".to_string()),
            true,
        )
        .field(
            "Ticket Count",
            raffle.ticket_count.separate_with_dots(),
            true,
        )
        .field(
            "Ticket Price",
            raffle.ticket_price.separate_with_dots(),
            true,
        )
        .field(
            "Payout",
            // 95% of the total ticket price. 5% because of tax.
            (raffle.ticket_count as f64 * raffle.ticket_price * 0.95_f64)
                .round()
                .separate_with_dots(),
            true,
        )
        .field(
            "Profit (Win)",
            profit_win
                .map(|x| x.round().separate_with_dots())
                .unwrap_or("Unknown".to_string()),
            true,
        )
        .field(
            "Profit (Lose)",
            profit_lose
                .map(|x| x.round().separate_with_dots())
                .unwrap_or("Unknown".to_string()),
            true,
        )
        .field(
            "Expected Value",
            expected_value
                .map(|x| x.round().separate_with_dots())
                .unwrap_or("Unknown".to_string()),
            true,
        )
        .footer(CreateEmbedFooter::new(format!(
            "RaffleID: {}",
            raffle.raffle_id
        ))))
}

fn parse_raffles(
    raffles: &[&Notification],
    char_id: i32,
) -> Result<Vec<EvEHypernetRaffle>, anyhow::Error> {
    let mut eve_raffles: Vec<EvEHypernetRaffle> = vec![];
    for raffle in raffles.iter() {
        let text = raffle.text.as_ref().unwrap();
        let parts: HashMap<String, String> = text
            .split('\n')
            .filter(|x| !x.is_empty())
            .map(|x| x.trim().splitn(2, ": "))
            .map(|x| x.map(|y| y.to_string()).collect::<Vec<_>>())
            .map(|x| (x[0].clone(), x[1].clone()))
            .collect();

        let owner_id = parts
            .get("owner_id")
            .ok_or(anyhow!("Missing owner_id"))?
            .parse::<i32>()?;
        let raffle_id = parts
            .get("raffle_id")
            .ok_or(anyhow!("Missing raffle_id"))?
            .clone();
        let location_id = parts
            .get("location_id")
            .ok_or(anyhow!("Missing location_id"))?
            .parse::<i32>()?;
        let ticket_price = parts
            .get("ticket_price")
            .ok_or(anyhow!("Missing ticket_price"))?
            .parse::<f64>()?;
        let ticket_count = parts
            .get("ticket_count")
            .ok_or(anyhow!("Missing ticket_count"))?
            .parse::<i32>()?;
        let type_id = parts
            .get("type_id")
            .ok_or(anyhow!("Missing type_id"))?
            .parse::<i32>()?;

        eve_raffles.push(EvEHypernetRaffle {
            location_id,
            owner_id,
            character_id: char_id,
            raffle_id,
            ticket_count,
            ticket_price,
            type_id,
            buy_price: None,
            sell_price: None,
            hypercore_sell_price: None,
            hypercore_buy_price: None,
            plex_price: None,
            status: HypernetRaffleStatus::Created,
            result: HypernetRaffleResult::None,
            created_at: chrono::DateTime::from_str(&raffle.timestamp)?,
        });
    }

    Ok(eve_raffles)
}

enum ProfitType {
    Winner,
    Loser,
}

fn calculate_profit(raffle: &EvEHypernetRaffle, status: ProfitType) -> Option<f64> {
    let item_value = raffle.ticket_count as f64 * raffle.ticket_price;
    let payout = item_value * 0.95; // 95% of the total ticket price. 5% because of tax.
    let plex_price = raffle.plex_price?;
    let required_cores = (item_value / (2.0 * plex_price)).floor();

    // If we win, we get the item back and the payout. But we spend 50% of the item_value on tickets
    // If we lose, we get nothing back. But we spend 50% of the item_value on tickets
    let profit = match status {
        Winner => {
            let total_income = raffle.buy_price? + payout;
            let total_expense = raffle.buy_price?
                + (required_cores * raffle.hypercore_sell_price?)
                + 0.5 * item_value;
            total_income - total_expense
        }
        Loser => {
            let total_expense = raffle.buy_price?
                + (required_cores * raffle.hypercore_sell_price?)
                + 0.5 * item_value;
            payout - total_expense
        }
    };

    Some(profit)
}

#[cfg(test)]
mod tests {
    use crate::cron::collect_hypernet_task::calculate_profit;
    use crate::cron::collect_hypernet_task::ProfitType::{Loser, Winner};
    use crate::database::hypernet_raffle_model::{
        EvEHypernetRaffle, HypernetRaffleResult, HypernetRaffleStatus,
    };

    #[tokio::test]
    async fn calculate_profit_test() {
        let raffle = EvEHypernetRaffle {
            owner_id: 1,
            character_id: 1,
            location_id: 1,
            raffle_id: "1".to_string(),
            ticket_count: 8,
            ticket_price: 10307323.0,
            type_id: 1,
            buy_price: Some(54730000.0),
            sell_price: Some(58430000.0),
            hypercore_buy_price: Some(304000.0),
            hypercore_sell_price: Some(327600.0),
            plex_price: Some(5736785.34),
            status: HypernetRaffleStatus::Created,
            result: HypernetRaffleResult::None,
            created_at: chrono::Utc::now(),
        };

        let profit = calculate_profit(&raffle, Winner).unwrap();
        dbg!(profit);
        assert!(profit > 0.0);

        let profit = calculate_profit(&raffle, Loser).unwrap();
        dbg!(profit);
        assert!(profit < 0.0);
    }
}
