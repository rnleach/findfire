CREATE TABLE IF NOT EXISTS clusters (
  cluster_id      INTEGER PRIMARY KEY AUTOINCREMENT,
  satellite       TEXT    NOT NULL,
  sector          TEXT    NOT NULL,
  start_time      INTEGER NOT NULL,  -- unix timestamp
  end_time        INTEGER NOT NULL,  -- unix timestamp
  lat             REAL    NOT NULL,
  lon             REAL    NOT NULL,
  power           REAL    NOT NULL,  -- megawatts
  max_temperature REAL    NOT NULL,  -- Kelvin
  area            REAL    NOT NULL,  -- square meters
  max_scan_angle  REAL    NOT NULL,  -- degrees
  pixels          BLOB    NOT NULL);

CREATE UNIQUE INDEX IF NOT EXISTS no_cluster_dups
  ON clusters (satellite, sector, start_time,
               end_time, lat, lon);

CREATE INDEX IF NOT EXISTS file_processed
  ON clusters (satellite, sector, start_time,
               end_time);

-- This table records files that have been processed, but 
-- did not contain any clusters.
CREATE TABLE IF NOT EXISTS no_clusters (
  satellite  TEXT    NOT NULL,
  sector     TEXT    NOT NULL,
  start_time INTEGER NOT NULL,
  end_time   INTEGER NOT NULL);

