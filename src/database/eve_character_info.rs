use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvECharacterInfo {
    pub discord_user_id: i64,
    pub character_id: i32,
    pub character_name: String,
    pub refresh_token: String,
}
