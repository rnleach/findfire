SELECT MAX(start_time) as latest FROM clusters WHERE satellite = ? AND sector = ?
UNION
SELECT MAX(start_time) as latest FROM no_clusters WHERE satellite = ? and sector = ?
ORDER BY latest DESC
LIMIT 1
