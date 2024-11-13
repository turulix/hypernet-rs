use crate::context::AppContext;
use crate::rest::api_error::ApiError;
use actix_web::http::StatusCode;
use actix_web::{get, web, HttpResponse, HttpResponseBuilder};
use log::error;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use sqlx::query_file;

#[derive(Deserialize, Serialize, Debug)]
struct QueryParams {
    code: String,
    state: String,
}

#[get("/callback")]
async fn callback(
    ctx: web::Data<AppContext>,
    params: web::Query<QueryParams>,
) -> Result<HttpResponseBuilder, ApiError> {
    let mut esi = ctx.esi.clone();
    let params = params.into_inner();

    let state: Vec<String> = params.state.split('-').map(|s| s.to_string()).collect();

    let discord_id = i64::from_str(&state[1]).map_err(|e| {
        error!("Error parsing Discord ID: {:?}", e);
        ApiError::new_with_title(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Authentication failed",
            "Failed to parse Discord ID",
        )
    })?;
    let esi_state = state[0].clone();

    drop(state);

    let auth = esi
        .authenticate(&params.code.clone(), None)
        .await
        .map_err(|e| {
            error!("Error authenticating: {:?}", e);
            ApiError::new_with_title(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Authentication failed",
                "Failed to authenticate with ESI",
            )
        })?;

    let auth = auth.ok_or_else(|| {
        ApiError::new_with_title(
            StatusCode::UNAUTHORIZED,
            "Authentication failed",
            "No authentication data returned",
        )
    })?;

    let char_name = auth.name;

    // The millisecond unix timestamp after which the access token expires, if present to Timestamp.
    // E is the access token expiration time in milliseconds.

    let char_id: i32 = auth
        .sub
        .split(':')
        .last()
        .ok_or(ApiError::new_with_title(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Authentication failed",
            "No character ID returned",
        ))?
        .parse()
        .map_err(|e| {
            error!("Error parsing character ID: {:?}", e);
            ApiError::new_with_title(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Authentication failed",
                "Failed to parse character ID",
            )
        })?;
    let refresh_token = esi.refresh_token.clone().ok_or_else(|| {
        error!("No refresh token returned");
        ApiError::new_with_title(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Authentication failed",
            "No refresh token returned",
        )
    })?;

    query_file!(
        "./sql/auth_requests/remove_esi_state.sql",
        esi_state,
        &discord_id
    ).execute(&ctx.postgres).await.map_err(|e| {
        error!("Error removing ESI state: {:?}", e);
        ApiError::new_with_title(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Authentication failed",
            "Failed to remove ESI state",
        )
    })?;

    query_file!(
        "./sql/eve_character/insert_eve_character_info.sql",
        char_id,
        discord_id,
        char_name,
        refresh_token
    ).execute(&ctx.postgres).await.map_err(|e| {
        error!("Error inserting character info: {:?}", e);
        ApiError::new_with_title(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Authentication failed",
            "Failed to insert character info",
        )
    })?;

    Ok(HttpResponse::Ok())
}
