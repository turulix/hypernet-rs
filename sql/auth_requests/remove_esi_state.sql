UPDATE auth_requests
SET esi_state = NULL
WHERE esi_state = $1 AND discord_user_id = $2;