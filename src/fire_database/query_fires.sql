SELECT id, last_observed, origin_lat, origin_lon, geometry
FROM fires
WHERE satellite = ?
ORDER BY id ASC
