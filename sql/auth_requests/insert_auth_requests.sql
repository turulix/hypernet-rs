INSERT INTO auth_requests (discord_user_id, esi_state)
VALUES ($1, $2)
ON CONFLICT (discord_user_id) DO UPDATE SET esi_state = $2;