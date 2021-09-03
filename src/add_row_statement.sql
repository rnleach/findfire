INSERT OR IGNORE INTO clusters
(
    satellite, 
    sector, 
    mid_point_time, 
    lat, 
    lon, 
    power, 
    radius, 
    cell_count
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)

