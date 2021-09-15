SELECT satellite, sector, start_time, lat, lon, power, perimeter, cell_count
FROM clusters 
WHERE satellite = ? 
ORDER BY start_time ASC
