use crate::{
    cluster::ClusterList,
    geo::{BoundingBox, Coord, Geo},
    pixel::PixelList,
    satellite::{Satellite, Sector},
};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, ToSql};
use std::{error::Error, path::Path};

/// Represents a connection to the database where ALL the information related to fires is stored.
pub struct FireDatabase {
    conn: Connection,
}

impl FireDatabase {
    /// Initialize a database.
    ///
    /// Initialize a database to make sure it exists and is set up properly. This should be run in
    /// the main thread before any other threads open a connection to the database to ensure
    /// consistency.
    pub fn initialize<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn Error>> {
        let path = path.as_ref();

        let _conn = Self::open_database_to_write(path)?;
        Ok(())
    }

    /// Open a connection to the database to store clusters, wildfires, and associations.
    pub fn connect<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let path = path.as_ref();

        let conn = Self::open_database_to_write(path)?;
        Ok(FireDatabase { conn })
    }

    fn open_database_to_write(path: &Path) -> Result<Connection, Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        // A 5-second busy time out is WAY too much. If we hit this something has gone terribly wrong.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        const QUERY: &str = include_str!("database/create_db.sql");
        conn.execute_batch(QUERY)?;

        Ok(conn)
    }

    /// Find the latest valid time in the database so you can safely skip anything older.
    pub fn newest_scan_start(
        &self,
        satellite: Satellite,
        sector: Sector,
    ) -> Result<DateTime<Utc>, Box<dyn Error>> {
        const QUERY: &str = include_str!("database/query_newest_start.sql");
        "SELECT MAX(start_time) FROM clusters WHERE satellite = ? AND sector = ?";
        let mut stmt = self.conn.prepare(QUERY)?;

        let res: DateTime<Utc> = stmt.query_row([satellite.name(), sector.name()], |row| {
            let timestamp: i64 = row.get(0)?;
            let naive = chrono::NaiveDateTime::from_timestamp(timestamp, 0);
            let value = DateTime::<Utc>::from_utc(naive, Utc);
            Ok(value)
        })?;

        Ok(res)
    }

    /// Get the next id number for a wildfire.
    pub fn next_wildfire_id(&self) -> Result<u64, Box<dyn Error>> {
        const QUERY: &str = "SELECT IFNULL(MAX(fire_id) + 1, 1) FROM fires";

        let mut stmt = self.conn.prepare(QUERY)?;
        let res: u64 = stmt.query_row([], |row| row.get(0))?;

        Ok(res)
    }

    /// Prepare to add cluster rows to the database.
    pub fn satfire_cluster_db_prepare_to_add(
        &self,
    ) -> Result<FireDatabaseAddCluster, Box<dyn Error>> {
        const ADD_CLUSTER_QUERY: &str = include_str!("database/add_cluster.sql");
        const ADD_NO_FIRE_QUERY: &str = include_str!("database/add_no_fire_cluster.sql");

        let add_cluster_stmt = self.conn.prepare(ADD_CLUSTER_QUERY)?;
        let add_no_fire_stmt = self.conn.prepare(ADD_NO_FIRE_QUERY)?;

        Ok(FireDatabaseAddCluster {
            add_cluster_stmt,
            add_no_fire_stmt,
            conn: &self.conn,
        })
    }

    /// Prepare to query the database if data from a satellite image is already in the database.
    pub fn prepare_to_query_present(
        &self,
    ) -> Result<FireDatabaseQueryClusterPresent, Box<dyn Error>> {
        const QUERY_CLUSTER: &str = include_str!("database/query_num_clusters_present.sql");
        const QUERY_NO_FIRE: &str = include_str!("database/query_no_fires.sql");

        let clusters_stmt = self.conn.prepare(QUERY_CLUSTER)?;
        let no_fire_stmt = self.conn.prepare(QUERY_NO_FIRE)?;

        Ok(FireDatabaseQueryClusterPresent {
            clusters_stmt,
            no_fire_stmt,
        })
    }

    /// Query clusters from the database.
    pub fn query_clusters<'a>(
        &'a self,
        sat: Option<Satellite>,
        sect: Option<Sector>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        area: BoundingBox,
    ) -> Result<FireDatabaseQueryClusters<'a>, Box<dyn Error>> {
        let sat_select = if let Some(sat) = sat {
            format!("AND satellite = '{}'", sat.name())
        } else {
            format!("")
        };

        let sector_select = if let Some(sect) = sect {
            format!("AND sector = '{}'", sect.name())
        } else {
            format!("")
        };

        let query = &format!(
            r#"SELECT
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

        Ok(FireDatabaseQueryClusters { stmt })
    }
}

pub struct FireDatabaseAddCluster<'a> {
    add_cluster_stmt: rusqlite::Statement<'a>,
    add_no_fire_stmt: rusqlite::Statement<'a>,
    conn: &'a Connection,
}

impl<'a> FireDatabaseAddCluster<'a> {
    /// Adds an entire ClusterList to the database.
    pub fn add(&mut self, clist: ClusterList) -> Result<(), Box<dyn Error>> {
        if clist.is_empty() {
            self.add_no_fire(clist)
        } else {
            self.add_clusters(clist)
        }
    }

    fn add_clusters(&mut self, clist: ClusterList) -> Result<(), Box<dyn Error>> {
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

    fn add_no_fire(&mut self, clist: ClusterList) -> Result<(), Box<dyn Error>> {
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

pub struct FireDatabaseQueryClusterPresent<'a> {
    clusters_stmt: rusqlite::Statement<'a>,
    no_fire_stmt: rusqlite::Statement<'a>,
}

impl<'a> FireDatabaseQueryClusterPresent<'a> {
    /// Check to see if an entry for these values already exists in the database.
    pub fn present(
        &mut self,
        satellite: Satellite,
        sector: Sector,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<bool, Box<dyn Error>> {
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

pub struct FireDatabaseQueryClusters<'a> {
    stmt: rusqlite::Statement<'a>,
}

impl<'a> FireDatabaseQueryClusters<'a> {
    /// Get an iterator over the rows
    pub fn rows(
        &mut self,
    ) -> Result<
        impl Iterator<Item = Result<FireDatabaseClusterRow, Box<dyn Error>>> + '_,
        Box<dyn Error>,
    > {
        Ok(self.stmt.query_and_then(
            [],
            |row| -> Result<FireDatabaseClusterRow, Box<dyn Error>> {
                let sat = match row.get_ref(0)? {
                    rusqlite::types::ValueRef::Text(txt) => {
                        let txt = unsafe { std::str::from_utf8_unchecked(txt) };
                        Satellite::string_contains_satellite(txt).ok_or("Invalid satellite")
                    }
                    _ => Err("satellite not text"),
                }?;

                let sector = match row.get_ref(1)? {
                    rusqlite::types::ValueRef::Text(txt) => {
                        let txt = unsafe { std::str::from_utf8_unchecked(txt) };
                        Sector::string_contains_sector(txt).ok_or("Invalid sector")
                    }
                    _ => Err("sector not text"),
                }?;

                let start: DateTime<Utc> =
                    DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(row.get(2)?, 0), Utc);
                let end: DateTime<Utc> =
                    DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(row.get(3)?, 0), Utc);
                let power: f64 = row.get(4)?;
                let max_temperature: f64 = row.get(5)?;
                let area: f64 = row.get(6)?;
                let scan_angle: f64 = row.get(7)?;
                let lat: f64 = row.get(8)?;
                let lon: f64 = row.get(9)?;
                let centroid = Coord { lat, lon };

                let pixels = match row.get_ref(10)? {
                    rusqlite::types::ValueRef::Blob(bytes) => {
                        let mut cursor = std::io::Cursor::new(bytes);
                        Ok(PixelList::binary_deserialize(&mut cursor))
                    }
                    _ => Err("Invalid type in pixels column"),
                }?;

                Ok(FireDatabaseClusterRow {
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
            },
        )?)
    }
}

/// All the data about a cluster retrieved from the database.
pub struct FireDatabaseClusterRow {
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

impl FireDatabaseClusterRow {}

pub struct FireDatabaseAddFire<'a> {
    stmt: rusqlite::Statement<'a>,
}

impl<'a> FireDatabaseAddFire<'a> {}

/*

/**
 * \brief Prepare to add rows to the fires database.
 *
 * \returns NULL or the 0 pointer on error.
 */
SFFiresDatabaseAddH satfire_fires_db_prepare_to_add(SFDatabaseH db);
SFFiresDatabaseAddH
satfire_fires_db_prepare_to_add(SFDatabaseH db)
{
    // TODO implement
    assert(false);
    return 0;
}

/**
 * \brief Finalize add transaction.
 *
 * \returns 0 if there is no error.
 */
int satfire_fires_db_finalize_add(SFFiresDatabaseAddH *stmt);
int
satfire_fires_db_finalize_add(SFFiresDatabaseAddH *stmt)
{
    // TODO implement
    assert(false);
    return 0;
}

/**
 * \brief Adds or updates a fire to the database.
 *
 * \returns the 0 on success and non-zero on error.
 */
int satfire_fires_db_add(SFFiresDatabaseAddH stmt, struct SFWildfire *fire);
int
satfire_fires_db_add(SFFiresDatabaseAddH stmt, struct SFWildfire *fire)
{
    // TODO implement
    assert(false);
    return 0;
}

*/

/*
struct SFPixelList *
satfire_cluster_db_satfire_cluster_row_steal_pixels(struct SFClusterRow *row)
{
    return g_steal_pointer(&row->pixels);
}
*/
