INSERT OR REPLACE INTO fires (
    fire_id,
    satellite,
    first_observed,
    last_observed,
    lat,
    lon,
    max_power,
    max_temperature,
    pixels)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)

