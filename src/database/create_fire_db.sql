CREATE TABLE IF NOT EXISTS fires (
  fire_id         INTEGER PRIMARY KEY AUTOINCREMENT,
  merged_into     INTEGER NOT NULL,
  satellite       TEXT    NOT NULL,
  first_observed  INTEGER NOT NULL,  --unix timestamp
  last_observed   INTEGER NOT NULL,  --unix timestamp
  lat             REAL    NOT NULL,
  lon             REAL    NOT NULL,
  max_power       REAL    NOT NULL,
  max_temperature REAL    NOT NULL,
  num_pixels      INTEGER NOT NULL,  -- number of pixels in the pixels object.
  pixels          BLOB    NOT NULL);

-- These are associations between fires and clusters.
CREATE TABLE IF NOT EXISTS associations (
  fire_id    INTEGER NOT NULL,
  cluster_id INTEGER NOT NULL,
  UNIQUE(fire_id, cluster_id));

