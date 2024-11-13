INSERT INTO notification_channel_map (discord_user_id, channel_id)
VALUES ($1, $2)
on conflict (discord_user_id) do update set channel_id = excluded.channel_id;