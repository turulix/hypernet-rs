CREATE type hypernet_raffle_status as ENUM ('Created', 'Expired', 'Finished');
CREATE type hypernet_raffle_result as ENUM ('None', 'Winner', 'Loser');

CREATE table auth_requests
(
    discord_user_id int8 not null primary key,
    esi_state       text unique
);

CREATE table eve_character_info
(
    character_id    int primary key                                 not null,
    discord_user_id int8 references auth_requests (discord_user_id) not null,
    character_name  text                                            not null,
    refresh_token   text                                            not null
);

CREATE TABLE notification_channel_map
(
    discord_user_id int8 primary key not null references auth_requests (discord_user_id),
    channel_id      int8
);

CREATE TABLE hypernet_raffles
(
    location_id  int                                                not null,
    owner_id     int                                                not null,
    character_id int references eve_character_info (character_id)   not null,
    raffle_id    text primary key                                   not null,
    ticket_count int                                                not null,
    ticket_price float8                                             not null,
    type_id      int                                                not null,
    status       hypernet_raffle_status                             not null,
    result       hypernet_raffle_result                             not null,
    created_at   TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP not null
);