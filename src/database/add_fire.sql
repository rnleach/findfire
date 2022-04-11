INSERT OR REPLACE INTO fires (
    fire_id,
    merged_into,
    satellite,
    first_observed,
    last_observed,
    lat,
    lon,
    max_power,
    max_temperature,
    num_pixels,
    pixels)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)

