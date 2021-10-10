CREATE TABLE IF NOT EXISTS clusters
(
  satellite  TEXT    NOT NULL,
  sector     TEXT    NOT NULL,
  start_time INTEGER NOT NULL,
  lat        REAL    NOT NULL,
  lon        REAL    NOT NULL,
  power      REAL    NOT NULL,
  geometry   BLOB    NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS no_cluster_dups 
  ON clusters (satellite, sector, start_time, lat, lon);
