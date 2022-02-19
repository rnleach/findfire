INSERT OR REPLACE INTO clusters (
  satellite,
  sector,
  start_time,
  end_time,
  lat,
  lon,
  power,
  max_temperature,
  area,
  max_scan_angle,
  pixels)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)

