WITH RECURSIVE
  find_mergers(x) AS (
    VALUES(3926759)
	UNION ALL
	SELECT fire_id FROM fires, find_mergers WHERE merged_into= find_mergers.x
  )
 SELECT fire_id FROM fires
 WHERE fire_id IN find_mergers
 ORDER BY fire_id ASC

