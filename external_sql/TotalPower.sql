DROP TABLE IF EXISTS fire_clust_assoc;

CREATE TEMPORARY TABLE IF NOT EXISTS fire_clust_assoc AS
SELECT SUBSTR(fires.id,1,6) as id, fires.id as fid, power, scan_start_time
FROM associations JOIN fires ON associations.fire_id = fires.id;

SELECT id, SUM(power) as total_power, scan_start_time
FROM fire_clust_assoc 
WHERE id = '000049'
GROUP BY scan_start_time
ORDER BY scan_start_time ASC;
