INSERT INTO hypernet_raffles(location_id, owner_id, character_id, raffle_id, ticket_count, ticket_price, type_id,
                             status, result, created_at, sell_price, buy_price, hypercore_buy_price,
                             hypercore_sell_price)
VALUES ($1,
        $2,
        $3,
        $4,
        $5,
        $6,
        $7,
        $8,
        $9,
        $10,
        $11,
        $12,
        $13,
        $14)
on conflict do nothing;