SELECT rowid, mid_point_time, lat, lon, power, perimeter 
FROM clusters 
WHERE satellite = ? 
ORDER BY mid_point_time ASC
