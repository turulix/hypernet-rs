INSERT INTO eve_character_info(character_id, discord_user_id, character_name, refresh_token)
values ($1, $2, $3, $4)
ON CONFLICT (character_id) DO UPDATE SET discord_user_id = $2,
                                         character_name  = $3,
                                         refresh_token   = $4;