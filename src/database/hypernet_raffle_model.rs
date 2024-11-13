use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Serialize, Debug, sqlx::Type, Clone)]
#[sqlx(type_name = "hypernet_raffle_status")]
pub enum HypernetRaffleStatus {
    Created,
    Expired,
    Finished,
}

impl Display for HypernetRaffleStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HypernetRaffleStatus::Created => write!(f, "Created"),
            HypernetRaffleStatus::Expired => write!(f, "Expired"),
            HypernetRaffleStatus::Finished => write!(f, "Finished"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, sqlx::Type, Clone)]
#[sqlx(type_name = "hypernet_raffle_result")]
pub enum HypernetRaffleResult {
    None,
    Winner,
    Loser,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct EvEHypernetRaffle {
    pub location_id: i32,
    pub owner_id: i32,
    pub character_id: i32,
    pub raffle_id: String,
    pub ticket_count: i32,
    pub ticket_price: f64,
    pub type_id: i32,
    pub status: HypernetRaffleStatus,
    pub result: HypernetRaffleResult,
    pub created_at: chrono::DateTime<Utc>,
}
