SELECT rowid, start_time, lat, lon, power, perimeter 
FROM clusters 
WHERE satellite = ? 
ORDER BY start_time ASC
