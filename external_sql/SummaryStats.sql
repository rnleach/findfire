DROP TABLE IF EXISTS fire_clust_assoc;

CREATE TEMPORARY TABLE IF NOT EXISTS fire_clust_assoc AS
SELECT SUBSTR(fires.id,1,6) as id, fires.id as fid, power, scan_start_time, origin_lat, origin_lon
FROM associations JOIN fires ON associations.fire_id = fires.id;

SElECT 
  COUNT(*) as count_obs, 
  MAX(power) as max_power, 
  id,
  origin_lat,
  origin_lon,
  (MAX(scan_start_time) - MIN(scan_start_time)) / 60 / 60 / 24 as duration_days
FROM fire_clust_assoc
GROUP BY id
ORDER BY duration_days DESC, max_power DESC;
