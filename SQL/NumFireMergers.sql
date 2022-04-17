SELECT merged_into, COUNT(merged_into) as cnt
FROM fires
WHERE merged_into <> 0 
GROUP BY merged_into
ORDER BY cnt DESC
LIMIT 10