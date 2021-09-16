SELECT id, last_observed, origin_lat, origin_lon, perimeter, next_child 
FROM fires
WHERE satellite = ?
ORDER BY id ASC
