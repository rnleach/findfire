SELECT satellite, sector, start_time, lat, lon, power, geometry
FROM clusters 
WHERE satellite = ? 
ORDER BY start_time ASC
