-- Add migration script here
START TRANSACTION;
ALTER table hypernet_raffles
    ADD column sell_price float8 null;
ALTER table hypernet_raffles
    ADD column buy_price float8 null;
ALTER table hypernet_raffles
    ADD column hypercore_buy_price float8 null;
ALTER table hypernet_raffles
    ADD column hypercore_sell_price float8 null;

COMMIT TRANSACTION;