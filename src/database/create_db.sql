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

CREATE TABLE IF NOT EXISTS no_fire (
  satellite  TEXT    NOT NULL,
  sector     TEXT    NOT NULL,
  start_time INTEGER NOT NULL,
  end_time   INTEGER NOT NULL);

CREATE TABLE IF NOT EXISTS fires (
  fire_id         INTEGER PRIMARY KEY AUTOINCREMENT,
  satellite       TEXT    NOT NULL,
  first_observed  INTEGER NOT NULL,  --unix timestamp
  last_observed   INTEGER NOT NULL,  --unix timestamp
  lat             REAL    NOT NULL,
  lon             REAL    NOT NULL,
  max_power       REAL    NOT NULL,
  max_temperature REAL    NOR NULL,
  pixels          BLOB    NOT NULL);

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS associations (
  fire_id    INTEGER NOT NULL,
  cluster_id INTEGER NOT NULL,
  FOREIGN KEY(fire_id)    REFERENCES fires(fire_id),
  FOREIGN KEY(cluster_id) REFERENCES clusers(cluster_id));

