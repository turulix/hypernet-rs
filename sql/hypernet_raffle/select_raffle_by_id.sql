SELECT location_id,
       owner_id,
       character_id,
       raffle_id,
       ticket_count,
       ticket_price,
       type_id,
       status as "status: HypernetRaffleStatus",
       result as "result: HypernetRaffleResult",
       created_at
FROM hypernet_raffles
WHERE raffle_id = $1;