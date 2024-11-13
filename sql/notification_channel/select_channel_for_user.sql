SELECT channel_id
FROM eve_character_info
         JOIN public.notification_channel_map ncm on eve_character_info.discord_user_id = ncm.discord_user_id
WHERE eve_character_info.character_id = $1