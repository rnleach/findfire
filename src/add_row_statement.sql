INSERT OR IGNORE INTO clusters
(
    satellite, 
    sector, 
    start_time, 
    lat, 
    lon, 
    power, 
    radius, 
    cell_count
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)

