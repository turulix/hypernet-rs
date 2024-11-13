INSERT INTO hypernet_raffles(location_id, owner_id, character_id, raffle_id, ticket_count, ticket_price, type_id,
                             status, result, created_at)
VALUES ($1,
        $2,
        $3,
        $4,
        $5,
        $6,
        $7,
        $8,
        $9,
        $10)
on conflict do nothing;