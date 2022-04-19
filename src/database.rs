use crate::{
    cluster::ClusterList,
    fire::{Fire, FireList},
    geo::{BoundingBox, Coord, Geo},
    pixel::PixelList,
    satellite::{Satellite, Sector},
    SatFireResult,
};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use log::{info, warn};
use rusqlite::{Connection, OpenFlags, ToSql};
use rustc_hash::FxHashMap as HashMap;
use std::path::Path;

/// Represents a connection to the database where ALL the information related to fires is stored.
pub struct ClusterDatabase {
    conn: Connection,
}

impl ClusterDatabase {
    /// Initialize a database.
    ///
    /// Initialize a database to make sure it exists and is set up properly. This should be run in
    /// the main thread before any other threads open a connection to the database to ensure
    /// consistency.
    pub fn initialize<P: AsRef<Path>>(path: P) -> SatFireResult<()> {
        let path = path.as_ref();

        let _conn = Self::open_database_to_write(path)?;
        Ok(())
    }

    /// Open a connection to the database to store clusters, wildfires, and associations.
    pub fn connect<P: AsRef<Path>>(path: P) -> SatFireResult<Self> {
        let path = path.as_ref();

        let conn = Self::open_database_to_write(path)?;
        Ok(ClusterDatabase { conn })
    }

    fn open_database_to_write(path: &Path) -> SatFireResult<Connection> {
        let conn = rusqlite::Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        // A 5-second busy time out is WAY too much. If we hit this something has gone terribly wrong.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        const QUERY: &str = include_str!("database/create_cluster_db.sql");
        conn.execute_batch(QUERY)?;

        Ok(conn)
    }

    /// Find the latest valid time in the database so you can safely skip anything older.
    pub fn newest_scan_start(
        &self,
        satellite: Satellite,
        sector: Sector,
    ) -> SatFireResult<DateTime<Utc>> {
        const QUERY: &str = include_str!("database/query_newest_cluster.sql");
        let mut stmt = self.conn.prepare(QUERY)?;

        let res: DateTime<Utc> = stmt.query_row(
            [
                satellite.name(),
                sector.name(),
                satellite.name(),
                sector.name(),
            ],
            |row| {
                let timestamp: i64 = row.get(0)?;
                let naive = chrono::NaiveDateTime::from_timestamp(timestamp, 0);
                let value = DateTime::<Utc>::from_utc(naive, Utc);
                Ok(value)
            },
        )?;

        Ok(res)
    }

    /// Prepare to add cluster rows to the database.
    pub fn prepare_to_add_clusters(&self) -> SatFireResult<ClusterDatabaseAddCluster> {
        const ADD_CLUSTER_QUERY: &str = include_str!("database/add_cluster.sql");
        const ADD_NO_FIRE_QUERY: &str = include_str!("database/add_no_cluster.sql");

        let add_cluster_stmt = self.conn.prepare(ADD_CLUSTER_QUERY)?;
        let add_no_fire_stmt = self.conn.prepare(ADD_NO_FIRE_QUERY)?;

        Ok(ClusterDatabaseAddCluster {
            add_cluster_stmt,
            add_no_fire_stmt,
            conn: &self.conn,
        })
    }

    /// Prepare to query the database if data from a satellite image is already in the database.
    pub fn prepare_to_query_clusters_present(
        &self,
    ) -> SatFireResult<ClusterDatabaseQueryClusterPresent> {
        const QUERY_CLUSTER: &str = include_str!("database/query_num_clusters_present.sql");
        const QUERY_NO_FIRE: &str = include_str!("database/query_no_clusters.sql");

        let clusters_stmt = self.conn.prepare(QUERY_CLUSTER)?;
        let no_fire_stmt = self.conn.prepare(QUERY_NO_FIRE)?;

        Ok(ClusterDatabaseQueryClusterPresent {
            clusters_stmt,
            no_fire_stmt,
        })
    }

    /// Query clusters from the database.
    pub fn query_clusters(
        &self,
        sat: Option<Satellite>,
        sect: Option<Sector>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        area: BoundingBox,
    ) -> SatFireResult<ClusterDatabaseQueryClusters<'_>> {
        let sat_select = if let Some(sat) = sat {
            format!("AND satellite = '{}'", sat.name())
        } else {
            String::new()
        };

        let sector_select = if let Some(sect) = sect {
            format!("AND sector = '{}'", sect.name())
        } else {
            String::new()
        };

        let query = &format!(
            r#"SELECT
                 rowid,
                 satellite,
                 sector,
                 start_time,
                 end_time,
                 power,
                 max_temperature,
                 area,
                 max_scan_angle,
                 lat,
                 lon,
                 pixels
               FROM clusters
               WHERE
                 start_time >= {} AND
                 end_time <= {} AND
                 lat >= {} AND lat <= {} AND
                 lon >= {} AND lon <= {} {} {}
               ORDER BY start_time ASC"#,
            start.timestamp(),
            end.timestamp(),
            area.ll.lat,
            area.ur.lat,
            area.ll.lon,
            area.ur.lon,
            sat_select,
            sector_select
        );

        let stmt = self.conn.prepare(query)?;

        Ok(ClusterDatabaseQueryClusters { stmt })
    }
}

pub struct ClusterDatabaseAddCluster<'a> {
    add_cluster_stmt: rusqlite::Statement<'a>,
    add_no_fire_stmt: rusqlite::Statement<'a>,
    conn: &'a Connection,
}

impl<'a> ClusterDatabaseAddCluster<'a> {
    /// Adds an entire ClusterList to the database.
    pub fn add(&mut self, clist: ClusterList) -> SatFireResult<()> {
        if clist.is_empty() {
            self.add_no_fire(clist)
        } else {
            self.add_clusters(clist)
        }
    }

    fn add_clusters(&mut self, clist: ClusterList) -> SatFireResult<()> {
        self.conn.execute("BEGIN TRANSACTION", [])?;

        let satellite = clist.satellite();
        let sector = clist.sector();
        let scan_start = clist.scan_start().timestamp();
        let scan_end = clist.scan_end().timestamp();

        for cluster in clist.take_clusters().into_iter() {
            let Coord { lat, lon } = cluster.centroid();
            let pixels = cluster.pixels().binary_serialize();
            let power = cluster.total_power();
            let maxt = cluster.max_temperature();
            let area = cluster.total_area();
            let angle = cluster.max_scan_angle();

            self.add_cluster_stmt.execute([
                &satellite.name() as &dyn ToSql,
                &sector.name(),
                &scan_start,
                &scan_end,
                &lat,
                &lon,
                &power,
                &maxt,
                &area,
                &angle,
                &pixels,
            ])?;
        }

        self.conn.execute("COMMIT", [])?;

        Ok(())
    }

    fn add_no_fire(&mut self, clist: ClusterList) -> SatFireResult<()> {
        let satellite = clist.satellite();
        let sector = clist.sector();
        let scan_start = clist.scan_start().timestamp();
        let scan_end = clist.scan_end().timestamp();

        self.add_no_fire_stmt.execute([
            &satellite.name() as &dyn ToSql,
            &sector.name(),
            &scan_start,
            &scan_end,
        ])?;

        Ok(())
    }
}

pub struct ClusterDatabaseQueryClusterPresent<'a> {
    clusters_stmt: rusqlite::Statement<'a>,
    no_fire_stmt: rusqlite::Statement<'a>,
}

impl<'a> ClusterDatabaseQueryClusterPresent<'a> {
    /// Check to see if an entry for these values already exists in the database.
    pub fn present(
        &mut self,
        satellite: Satellite,
        sector: Sector,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> SatFireResult<bool> {
        let start = start.timestamp();
        let end = end.timestamp();

        let num_clusters: i64 = self.clusters_stmt.query_row(
            [
                &satellite.name() as &dyn ToSql,
                &sector.name(),
                &start,
                &end,
            ],
            |row| row.get(0),
        )?;

        if num_clusters <= 0 {
            let no_fire: i64 = self.no_fire_stmt.query_row(
                [
                    &satellite.name() as &dyn ToSql,
                    &sector.name(),
                    &start,
                    &end,
                ],
                |row| row.get(0),
            )?;

            // This satellite, sector, start, end time group was processessed and there were no
            // clusters found, so it is present in the database, just with no clusters.
            Ok(no_fire > 0)
        } else {
            // There was more than 0 clusters in the database, so YES, this satellite, sector,
            // start, and end time group was processessed.
            Ok(true)
        }
    }
}

pub struct ClusterDatabaseQueryClusters<'a> {
    stmt: rusqlite::Statement<'a>,
}

impl<'a> ClusterDatabaseQueryClusters<'a> {
    /// Get an iterator over the rows
    pub fn rows(
        &mut self,
    ) -> SatFireResult<impl Iterator<Item = SatFireResult<ClusterDatabaseClusterRow>> + '_> {
        Ok(self.stmt.query_and_then([], query_row_to_cluster_row)?)
    }
}

/// All the data about a cluster retrieved from the database.
#[derive(Debug, Clone)]
pub struct ClusterDatabaseClusterRow {
    pub rowid: u64,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub power: f64,
    pub max_temperature: f64,
    pub area: f64,
    pub scan_angle: f64,
    pub centroid: Coord,
    pub sector: Sector,
    pub sat: Satellite,
    pub pixels: PixelList,
}

impl ClusterDatabaseClusterRow {}

/// Represents a connection to the database where ALL the information related to fires is stored.
pub struct FiresDatabase {
    conn: Connection,
}

impl FiresDatabase {
    /// Initialize a database.
    ///
    /// Initialize a database to make sure it exists and is set up properly. This should be run in
    /// the main thread before any other threads open a connection to the database to ensure
    /// consistency.
    pub fn initialize<P: AsRef<Path>>(path: P) -> SatFireResult<()> {
        let path = path.as_ref();

        let _conn = Self::open_database_to_write(path)?;
        Ok(())
    }

    /// Open a connection to the database to store clusters, wildfires, and associations.
    pub fn connect<P: AsRef<Path>>(path: P) -> SatFireResult<Self> {
        let path = path.as_ref();

        let conn = Self::open_database_to_write(path)?;
        Ok(Self { conn })
    }

    fn open_database_to_write(path: &Path) -> SatFireResult<Connection> {
        let conn = rusqlite::Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        // A 5-second busy time out is WAY too much. If we hit this something has gone terribly wrong.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        const QUERY: &str = include_str!("database/create_fire_db.sql");
        conn.execute_batch(QUERY)?;

        Ok(conn)
    }

    /// Get the next id number for a wildfire.
    pub fn next_wildfire_id(&self) -> SatFireResult<u64> {
        const QUERY: &str = "SELECT IFNULL(MAX(fire_id) + 1, 1) FROM fires";

        let mut stmt = self.conn.prepare(QUERY)?;
        let res: u64 = stmt.query_row([], |row| row.get(0))?;

        Ok(res)
    }

    /// Get the most recent start time
    pub fn last_observed(&self, sat: Satellite) -> Option<DateTime<Utc>> {
        self.conn
            .query_row(
                "SELECT MAX(last_observed) FROM fires WHERE satellite = ?",
                [sat.name()],
                |row| row.get::<_, i64>(0),
            )
            .map(|time_stamp| {
                DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(time_stamp, 0), Utc)
            })
            .ok()
    }

    /// Get the fires that are still going.
    pub fn ongoing_fires(&self, sat: Satellite) -> SatFireResult<FireList> {
        let latest = match self.last_observed(sat) {
            Some(ts) => ts,
            None => return Ok(FireList::new()),
        };

        let earliest = latest - Duration::days(175);

        info!(target: sat.name(), "Latest fire observation => {}", latest);

        const QUERY: &str = include_str!("database/query_most_recent_fires.sql");
        let mut stmt = self.conn.prepare(QUERY)?;

        let mut fires = FireList::new();

        stmt.query_and_then(
            [&earliest.timestamp() as &dyn ToSql, &sat.name()],
            |row| -> SatFireResult<Fire> {
                let id: u64 = u64::try_from(row.get::<_, i64>(0)?)?;

                let sat = match row.get_ref(1)? {
                    rusqlite::types::ValueRef::Text(txt) => {
                        let txt = unsafe { std::str::from_utf8_unchecked(txt) };
                        Satellite::string_contains_satellite(txt).ok_or("Invalid sattelite")
                    }
                    _ => Err("sattelite not text"),
                }?;

                let first_observed: DateTime<Utc> =
                    DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(row.get(2)?, 0), Utc);
                let last_observed: DateTime<Utc> =
                    DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(row.get(3)?, 0), Utc);

                let max_power: f64 = row.get(4)?;
                let max_temperature: f64 = row.get(5)?;

                let area = match row.get_ref(6)? {
                    rusqlite::types::ValueRef::Blob(bytes) => {
                        let mut cursor = std::io::Cursor::new(bytes);
                        Ok(PixelList::binary_deserialize(&mut cursor))
                    }
                    _ => Err("Invalid type in pixels column"),
                }?;

                Ok(Fire::new(
                    first_observed,
                    last_observed,
                    max_power,
                    max_temperature,
                    id,
                    area,
                    sat,
                    0,
                ))
            },
        )?
        .filter_map(|res| match res {
            Ok(fire) => Some(fire),
            Err(err) => {
                warn!(target: sat.name(), "Error retrieving fire - {}", err);
                None
            }
        })
        .for_each(|fire| fires.add_fire(fire));

        let mut waste = FireList::default();

        info!(target: sat.name(), "Retrieved {} fires from database.", fires.len());

        fires.drain_stale_fires(&mut waste, latest);

        info!(target: sat.name(), "Retrieved {} fires from database after filtering out stale fires.",
            fires.len());

        Ok(fires)
    }

    /// Add fires and associations to clusters to the database.
    pub fn prepare_to_add_fires(&self) -> SatFireResult<FiresDatabaseAddFire> {
        const FIRE_QUERY: &str = include_str!("database/add_fire.sql");
        const ASSOC_QUERY: &str = include_str!("database/add_association.sql");

        let fire_stmt = self.conn.prepare(FIRE_QUERY)?;
        let assoc_stmt = self.conn.prepare(ASSOC_QUERY)?;
        let associations = HashMap::default();

        Ok(FiresDatabaseAddFire {
            conn: &self.conn,
            fire_stmt,
            assoc_stmt,
            associations,
        })
    }

    /// Query fires from the database
    pub fn query_fires(
        &self,
        sat: Option<Satellite>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        area: BoundingBox,
    ) -> SatFireResult<FiresDatabaseQueryFires<'_>> {
        let sat_select = if let Some(sat) = sat {
            format!("AND satellite = '{}'", sat.name())
        } else {
            String::new()
        };

        let query = &format!(
            r#"SELECT
                 fire_id,
                 merged_into,
                 satellite,
                 first_observed,
                 last_observed,
                 max_power,
                 max_temperature,
                 pixels
               FROM fires
               WHERE
                 ((first_observed <= {} AND last_observed >= {})
                 OR (first_observed >= {} AND first_observed <= {})
                 OR (last_observed >= {} AND last_observed <= {}))
                 AND
                 lat >= {} AND lat <= {} AND
                 lon >= {} AND lon <= {} {} 
               ORDER BY first_observed ASC"#,
            start.timestamp(),
            end.timestamp(),
            start.timestamp(),
            end.timestamp(),
            start.timestamp(),
            end.timestamp(),
            area.ll.lat,
            area.ur.lat,
            area.ll.lon,
            area.ur.lon,
            sat_select,
        );

        let stmt = self.conn.prepare(query)?;

        Ok(FiresDatabaseQueryFires { stmt })
    }
}

pub struct FiresDatabaseAddFire<'a> {
    conn: &'a rusqlite::Connection,
    fire_stmt: rusqlite::Statement<'a>,
    assoc_stmt: rusqlite::Statement<'a>,
    associations: HashMap<u64, Vec<u64>>,
}

impl<'a> FiresDatabaseAddFire<'a> {
    /// Add a list of fires to the database.
    pub fn add_fires(&mut self, fires: &FireList) -> SatFireResult<()> {
        let mut ids = Vec::with_capacity(fires.len());

        self.conn.execute("BEGIN TRANSACTION", [])?;

        for fire in fires.iter().filter(|f| f.duration() > Duration::hours(1)) {
            ids.push(fire.id());

            let Coord { lat, lon } = fire.centroid();
            let pixels = fire.pixels().binary_serialize();

            self.fire_stmt.execute([
                &fire.id() as &dyn ToSql,
                &fire.merged_into(),
                &fire.satellite().name(),
                &fire.first_observed().timestamp(),
                &fire.last_observed().timestamp(),
                &lat,
                &lon,
                &fire.max_power(),
                &fire.max_temperature(),
                &fire.pixels().len(),
                &pixels,
            ])?;
        }

        for id in ids {
            if let Some(cluster_ids) = self.associations.remove(&id) {
                for cluster_id in cluster_ids {
                    self.assoc_stmt.execute([id, cluster_id])?;
                }
            }
        }
        self.conn.execute("COMMIT", [])?;

        Ok(())
    }

    /// Add associations.
    pub fn add_association(&mut self, fireid: u64, clusterid: u64) {
        let cluster_ids = self.associations.entry(fireid).or_insert(vec![]);
        cluster_ids.push(clusterid);
    }
}

pub struct FiresDatabaseQueryFires<'a> {
    stmt: rusqlite::Statement<'a>,
}

impl<'a> FiresDatabaseQueryFires<'a> {
    /// Get an iterator over the rows
    pub fn rows(&mut self) -> SatFireResult<impl Iterator<Item = SatFireResult<Fire>> + '_> {
        Ok(self.stmt.query_and_then([], |row| -> SatFireResult<Fire> {
            let id: u64 = u64::try_from(row.get::<_, i64>(0)?)?;

            let merged_into: u64 = u64::try_from(row.get::<_, i64>(1)?)?;

            let sat = match row.get_ref(2)? {
                rusqlite::types::ValueRef::Text(txt) => {
                    let txt = unsafe { std::str::from_utf8_unchecked(txt) };
                    Satellite::string_contains_satellite(txt).ok_or("Invalid sattelite")
                }
                _ => Err("sattelite not text"),
            }?;

            let first_observed: DateTime<Utc> =
                DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(row.get(3)?, 0), Utc);
            let last_observed: DateTime<Utc> =
                DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(row.get(4)?, 0), Utc);

            let max_power: f64 = row.get(5)?;
            let max_temperature: f64 = row.get(6)?;

            let area = match row.get_ref(7)? {
                rusqlite::types::ValueRef::Blob(bytes) => {
                    let mut cursor = std::io::Cursor::new(bytes);
                    Ok(PixelList::binary_deserialize(&mut cursor))
                }
                _ => Err("Invalid type in pixels column"),
            }?;

            Ok(Fire::new(
                first_observed,
                last_observed,
                max_power,
                max_temperature,
                id,
                area,
                sat,
                merged_into,
            ))
        })?)
    }
}

pub struct JointFiresClusterDatabases {
    conn: Connection,
}

impl JointFiresClusterDatabases {
    pub fn connect<P1: AsRef<Path>, P2: AsRef<Path>>(
        clusters_db: P1,
        fires_db: P2,
    ) -> SatFireResult<Self> {
        let fires_path = fires_db.as_ref();
        let attach_clusters = format!(
            "ATTACH DATABASE \"{}\" AS ff",
            clusters_db.as_ref().display()
        );

        let conn = FiresDatabase::open_database_to_write(fires_path)?;
        conn.execute(&attach_clusters, [])?;

        Ok(JointFiresClusterDatabases { conn })
    }

    pub fn single_fire_query(&self) -> SatFireResult<JointQuerySingleFire> {
        let stmt = self.conn.prepare_cached(include_str!(
            "database/single_fire_clusters_time_series.sql"
        ))?;

        Ok(JointQuerySingleFire { stmt })
    }
}

pub struct JointQuerySingleFire<'a> {
    stmt: rusqlite::CachedStatement<'a>,
}

impl<'a> JointQuerySingleFire<'a> {
    /// Get an iterator over the rows
    pub fn run(
        &mut self,
        fire_id: u64,
    ) -> SatFireResult<impl Iterator<Item = SatFireResult<ClusterDatabaseClusterRow>> + '_> {
        Ok(self
            .stmt
            .query_and_then([fire_id], query_row_to_cluster_row)?)
    }
}

fn query_row_to_cluster_row(row: &rusqlite::Row) -> SatFireResult<ClusterDatabaseClusterRow> {
    let rowid: u64 = u64::try_from(row.get::<_, i64>(0)?)?;
    let sat = match row.get_ref(1)? {
        rusqlite::types::ValueRef::Text(txt) => {
            let txt = unsafe { std::str::from_utf8_unchecked(txt) };
            Satellite::string_contains_satellite(txt).ok_or("Invalid satellite")
        }
        _ => Err("satellite not text"),
    }?;

    let sector = match row.get_ref(2)? {
        rusqlite::types::ValueRef::Text(txt) => {
            let txt = unsafe { std::str::from_utf8_unchecked(txt) };
            Sector::string_contains_sector(txt).ok_or("Invalid sector")
        }
        _ => Err("sector not text"),
    }?;

    let start: DateTime<Utc> =
        DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(row.get(3)?, 0), Utc);
    let end: DateTime<Utc> =
        DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(row.get(4)?, 0), Utc);
    let power: f64 = row.get(5)?;
    let max_temperature: f64 = row.get(6)?;
    let area: f64 = row.get(7)?;
    let scan_angle: f64 = row.get(8)?;
    let lat: f64 = row.get(9)?;
    let lon: f64 = row.get(10)?;
    let centroid = Coord { lat, lon };

    let pixels = match row.get_ref(11)? {
        rusqlite::types::ValueRef::Blob(bytes) => {
            let mut cursor = std::io::Cursor::new(bytes);
            Ok(PixelList::binary_deserialize(&mut cursor))
        }
        _ => Err("Invalid type in pixels column"),
    }?;

    Ok(ClusterDatabaseClusterRow {
        rowid,
        sat,
        sector,
        start,
        end,
        power,
        max_temperature,
        area,
        scan_angle,
        centroid,
        pixels,
    })
}
