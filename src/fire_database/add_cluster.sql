INSERT OR IGNORE INTO clusters
(
    satellite, 
    sector, 
    start_time, 
    lat, 
    lon, 
    power, 
    cell_count,
    perimeter
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)

