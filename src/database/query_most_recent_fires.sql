SELECT 
  fire_id, 
  satellite,
  first_observed, 
  last_observed, 
  max_power, 
  max_temperature, 
  pixels 
FROM fires 
WHERE last_observed > ? AND satellite = ? AND merged_into = 0
