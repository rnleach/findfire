/*! Methods and types to support querying the clusters database. */
use crate::{
    cluster::Cluster,
    satellite::{Satellite, Sector},
};
use chrono::NaiveDateTime;
use geo::{point, MultiPolygon, Point};
use rusqlite::ToSql;
use std::{error::Error, path::Path, str::FromStr};

pub struct ClustersDatabase {
    db: rusqlite::Connection,
}

impl ClustersDatabase {
    pub fn connect<P: AsRef<Path>>(path_to_db: P) -> Result<Self, Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            path_to_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        conn.execute_batch(include_str!("cluster_database/create_cluster_db.sql"))?;

        Ok(ClustersDatabase { db: conn })
    }

    pub fn add_cluster_handle(&self) -> Result<AddClustersTransaction, Box<dyn Error>> {
        Ok(AddClustersTransaction {
            buffer: Vec::with_capacity(BUFFER_CAPACITY),
            db: &self.db,
        })
    }

    pub fn find_latest_cluster(
        &self,
        satellite: Satellite,
        sector: Sector,
    ) -> Result<NaiveDateTime, Box<dyn Error>> {
        let satellite = Into::<&'static str>::into(satellite);
        let sector = Into::<&'static str>::into(sector);

        let latest: NaiveDateTime = self.db.query_row(
            include_str!("cluster_database/find_latest_cluster.sql"),
            &[satellite, sector],
            |row| row.get(0),
        )?;

        Ok(latest)
    }

    pub fn cluster_query_handle(&self) -> Result<ClusterQuery, Box<dyn Error>> {
        let stmt = self
            .db
            .prepare(include_str!("cluster_database/get_clusters.sql"))?;
        Ok(ClusterQuery(stmt))
    }
}

//pub struct ClusterQuery<'a>(rusqlite::Statement<'a>);
pub struct ClusterQuery<'a>(rusqlite::Statement<'a>);

impl<'a> ClusterQuery<'a> {
    pub fn records_for(
        &mut self,
        satellite: Satellite,
    ) -> Result<impl Iterator<Item = Cluster> + '_, Box<dyn Error>> {
        let rows = self
            .0
            .query_and_then(&[Into::<&'static str>::into(satellite)], |row| {
                let satellite = Satellite::from_str(row.get_ref(0)?.as_str()?).unwrap();
                let sector = Sector::from_str(row.get_ref(1)?.as_str()?).unwrap();
                let scan_start_time: NaiveDateTime =
                    chrono::NaiveDateTime::from_timestamp(row.get::<_, i64>(2)?, 0);
                let lat: f64 = row.get(3)?;
                let lon: f64 = row.get(4)?;
                let centroid = point!(x: lon, y: lat);
                let power: f64 = row.get(5)?;

                let pblob = row.get_ref(6)?.as_blob()?;

                let perimeter: MultiPolygon<f64> =
                    bincode::deserialize(pblob).map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(Cluster {
                    satellite,
                    sector,
                    scan_start_time,
                    power,
                    perimeter,
                    centroid,
                })
            })?
            .filter_map(|res: Result<_, rusqlite::Error>| res.ok());

        Ok(rows)
    }
}

//pub struct AddClustersTransaction<'a>(rusqlite::Statement<'a>, &'a rusqlite::Connection);
pub struct AddClustersTransaction<'a> {
    buffer: Vec<(
        Satellite,
        Sector,
        NaiveDateTime,
        Point<f64>,
        f64,
        MultiPolygon<f64>,
    )>,
    db: &'a rusqlite::Connection,
}

const BUFFER_CAPACITY: usize = 1_000;

impl<'a> AddClustersTransaction<'a> {
    pub fn add_cluster(
        &mut self,
        satellite: Satellite,
        sector: Sector,
        scan_start: NaiveDateTime,
        centroid: Point<f64>,
        power: f64,
        perimeter: MultiPolygon<f64>,
    ) -> Result<(), Box<dyn Error>> {
        self.buffer
            .push((satellite, sector, scan_start, centroid, power, perimeter));

        if self.buffer.len() >= BUFFER_CAPACITY {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        log::debug!("Flushing clusters.");
        self.db.execute_batch("BEGIN;")?;
        let mut stmt = self
            .db
            .prepare(include_str!("cluster_database/add_cluster.sql"))?;

        for (satellite, sector, scan_start, centroid, power, perimeter) in self.buffer.drain(..) {
            let lon = centroid.x();
            let lat = centroid.y();

            let perimeter = bincode::serialize(&perimeter)?;
            let _ = stmt.execute([
                &Into::<&'static str>::into(satellite) as &dyn ToSql,
                &Into::<&'static str>::into(sector),
                &scan_start.timestamp(),
                &lat,
                &lon,
                &power,
                &perimeter,
            ])?;
        }

        self.db.execute_batch("COMMIT;")?;

        Ok(())
    }
}

impl<'a> Drop for AddClustersTransaction<'a> {
    fn drop(&mut self) {
        log::debug!("Dropping AddClustersTransaction");
        self.flush().unwrap();
    }
}
