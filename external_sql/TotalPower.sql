DROP TABLE IF EXISTS fire_clust_assoc;

CREATE TEMPORARY TABLE IF NOT EXISTS fire_clust_assoc AS
SELECT cluster_row_id, SUBSTR(fires.id,1,6) as id, fires.id as fid
FROM associations JOIN fires ON associations.fire_id = fires.id;

SELECT start_time, SUM(power) as total_power
FROM fire_clust_assoc JOIN clusters ON fire_clust_assoc.cluster_row_id = clusters.rowid
WHERE id = '049555'
GROUP BY start_time
ORDER BY start_time ASC;
