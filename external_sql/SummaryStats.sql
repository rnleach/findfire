DROP TABLE IF EXISTS fire_clust_assoc;

CREATE TEMPORARY TABLE IF NOT EXISTS fire_clust_assoc AS
SELECT cluster_row_id, SUBSTR(fires.id,1,6) as id, origin_lat, origin_lon
FROM associations JOIN fires ON associations.fire_id = fires.id;

SElECT 
  COUNT(*) as count_obs, 
  MAX(clusters.power) as max_power, 
  SUBSTR(fire_clust_assoc.id, 1, 6) as origin_id,
  fire_clust_assoc.origin_lat,
  fire_clust_assoc.origin_lon,
  (MAX(clusters.start_time) - MIN(clusters.start_time)) / 60 / 60 / 24 as duration_days
FROM clusters 
  JOIN fire_clust_assoc 
    ON clusters.rowid = fire_clust_assoc.cluster_row_id
GROUP BY origin_id
ORDER BY duration_days DESC;
