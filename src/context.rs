use rfesi::prelude::Esi;
use serenity::all::Http;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppContext {
    pub esi: Esi,
    pub postgres: sqlx::PgPool,
} // User data, which is stored and accessible in all command invocations

#[derive(Clone)]
pub struct CronAppContext {
    pub esi: Esi,
    pub postgres: sqlx::PgPool,
    pub discord_http: Arc<Http>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, AppContext, Error>;
