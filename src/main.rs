mod commands;
mod context;
mod cron;
mod database;
mod handler;
mod rest;

use crate::commands::change_notification_channel::change_notification_channel;
use crate::commands::help::help;
use crate::context::{AppContext, CronAppContext};
use crate::cron::start_cron;
use crate::handler::event_handler;
use actix_web::{web, App, HttpServer};
use commands::auth::auth;
use commands::register::register;
use log::{error, info};
use poise::builtins::{register_globally, register_in_guild};
use poise::Framework;
use rfesi::prelude::EsiBuilder;
use serenity::all::{GatewayIntents, GuildId, UserId};
use serenity::Client;
use sqlx::postgres::PgTypeInfo;
use sqlx::PgPool;
use std::collections::HashSet;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let database = PgPool::connect(&env::var("DATABASE_URL")?).await?;
    PgTypeInfo::with_name("hypernet_raffle_status");
    PgTypeInfo::with_name("hypernet_raffle_result");

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let debug = env::var("DEBUG").is_ok();

    let esi_client_id =
        env::var("ESI_CLIENT_ID").expect("Expected an ESI client ID in the environment");
    let esi_client_secret =
        env::var("ESI_CLIENT_SECRET").expect("Expected an ESI client secret in the environment");

    let scopes = [
        "publicData",
        "esi-calendar.respond_calendar_events.v1",
        "esi-calendar.read_calendar_events.v1",
        "esi-location.read_location.v1",
        "esi-location.read_ship_type.v1",
        "esi-mail.organize_mail.v1",
        "esi-mail.read_mail.v1",
        "esi-mail.send_mail.v1",
        "esi-skills.read_skills.v1",
        "esi-skills.read_skillqueue.v1",
        "esi-wallet.read_character_wallet.v1",
        "esi-search.search_structures.v1",
        "esi-clones.read_clones.v1",
        "esi-characters.read_contacts.v1",
        "esi-universe.read_structures.v1",
        "esi-bookmarks.read_character_bookmarks.v1",
        "esi-killmails.read_killmails.v1",
        "esi-corporations.read_corporation_membership.v1",
        "esi-assets.read_assets.v1",
        "esi-planets.manage_planets.v1",
        "esi-fleets.read_fleet.v1",
        "esi-fleets.write_fleet.v1",
        "esi-ui.open_window.v1",
        "esi-ui.write_waypoint.v1",
        "esi-characters.write_contacts.v1",
        "esi-fittings.read_fittings.v1",
        "esi-fittings.write_fittings.v1",
        "esi-markets.structure_markets.v1",
        "esi-corporations.read_structures.v1",
        "esi-characters.read_loyalty.v1",
        "esi-characters.read_opportunities.v1",
        "esi-characters.read_medals.v1",
        "esi-characters.read_standings.v1",
        "esi-characters.read_agents_research.v1",
        "esi-industry.read_character_jobs.v1",
        "esi-markets.read_character_orders.v1",
        "esi-characters.read_blueprints.v1",
        "esi-characters.read_corporation_roles.v1",
        "esi-location.read_online.v1",
        "esi-contracts.read_character_contracts.v1",
        "esi-clones.read_implants.v1",
        "esi-characters.read_fatigue.v1",
        "esi-killmails.read_corporation_killmails.v1",
        "esi-corporations.track_members.v1",
        "esi-wallet.read_corporation_wallets.v1",
        "esi-characters.read_notifications.v1",
        "esi-corporations.read_divisions.v1",
        "esi-corporations.read_contacts.v1",
        "esi-assets.read_corporation_assets.v1",
        "esi-corporations.read_titles.v1",
        "esi-corporations.read_blueprints.v1",
        "esi-bookmarks.read_corporation_bookmarks.v1",
        "esi-contracts.read_corporation_contracts.v1",
        "esi-corporations.read_standings.v1",
        "esi-corporations.read_starbases.v1",
        "esi-industry.read_corporation_jobs.v1",
        "esi-markets.read_corporation_orders.v1",
        "esi-corporations.read_container_logs.v1",
        "esi-industry.read_character_mining.v1",
        "esi-industry.read_corporation_mining.v1",
        "esi-planets.read_customs_offices.v1",
        "esi-corporations.read_facilities.v1",
        "esi-corporations.read_medals.v1",
        "esi-characters.read_titles.v1",
        "esi-alliances.read_contacts.v1",
        "esi-characters.read_fw_stats.v1",
        "esi-corporations.read_fw_stats.v1",
    ];

    let mut esi = EsiBuilder::new()
        .user_agent("Leira Saint")
        .client_id(&esi_client_id)
        .client_secret(&esi_client_secret)
        .callback_url("http://localhost:3000/callback")
        .scope(&scopes.join(" ").to_string())
        .build()
        .expect("Failed to build ESI client");
    esi.update_spec().await?;

    let intents = GatewayIntents::non_privileged();

    let options = poise::FrameworkOptions {
        commands: vec![help(), auth(), register(), change_notification_channel()],
        allowed_mentions: None,
        initialize_owners: true,
        event_handler: |ctx, event, framework, data| {
            Box::pin(event_handler(ctx, event, framework, data))
        },
        owners: HashSet::from_iter([UserId::new(262702226693160970)]),
        ..Default::default()
    };

    let data = AppContext {
        esi,
        postgres: database,
    };

    let data_cloned = data.clone();
    let framework = Framework::builder()
        .options(options)
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                info!("Logged in as {}", ready.user.name);

                let guild = GuildId::new(1002206872096473138);
                if debug {
                    register_in_guild(ctx, &framework.options().commands, guild).await?;
                } else {
                    register_globally(ctx, &framework.options().commands).await?;

                    let commands = guild.get_commands(&ctx.http).await?;
                    for x in commands {
                        guild.delete_command(&ctx.http, x.id).await?;
                    }
                }

                Ok(data_cloned)
            })
        })
        .initialize_owners(true)
        .build();

    let mut client = Client::builder(token, intents)
        .framework(framework)
        .await
        .expect("Error creating client");

    let http_context = data.clone();
    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(http_context.clone()))
            .service(rest::callback::callback)
    })
    .bind(("0.0.0.0", 3000))?;

    let cron_context = CronAppContext {
        esi: data.esi.clone(),
        postgres: data.postgres.clone(),
        discord_http: client.http.clone(),
    };

    tokio::select! {
        _ = http_server.run() => {
            error!("An error occurred while running the HTTP server");
        }
        Err(why) = client.start_autosharded() => {
            error!("An error occurred while running the client: {:?}", why);
        }
        _ = start_cron(cron_context) => {
            error!("An error occurred while running the cron tasks");
        }
    }

    Ok(())
}
