SELECT 
    clusters.cluster_id,
    clusters.satellite,
    clusters.sector,
    clusters.start_time,
    clusters.end_time,
    clusters.power,
    clusters.max_temperature,
    clusters.area,
    clusters.max_scan_angle,
    clusters.lat,
    clusters.lon,
    clusters.pixels
FROM associations JOIN ff.clusters ON ff.clusters.cluster_id = associations.cluster_id
WHERE associations.fire_id in (
     WITH RECURSIVE
          find_mergers(x) AS (
	           VALUES(?)
		       UNION ALL
		       SELECT fire_id FROM fires, find_mergers WHERE merged_into = find_mergers.x
          )
	      SELECT fire_id FROM fires
	     WHERE fire_id IN find_mergers
	     ORDER BY fire_id ASC
)
ORDER BY start_time ASC
