INSERT OR REPLACE INTO fires
(
    id, 
    satellite,
    last_observed, 
    origin_lat, 
    origin_lon, 
    geometry
) VALUES (?, ?, ?, ?, ?, ?)

