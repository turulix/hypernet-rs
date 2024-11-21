SELECT location_id,
       owner_id,
       character_id,
       raffle_id,
       ticket_count,
       ticket_price,
       type_id,
       status as "status: HypernetRaffleStatus",
       result as "result: HypernetRaffleResult",
       created_at,
       buy_price,
       sell_price,
       hypercore_buy_price,
       hypercore_sell_price
FROM hypernet_raffles
WHERE raffle_id = $1;