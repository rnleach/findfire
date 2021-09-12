/*! Methods and types to support querying the clusters table of the database. */

use std::error::Error;

use chrono::NaiveDateTime;
use geo::{point, Point, Polygon};
use rusqlite::ToSql;

impl super::FiresDatabase {
    pub fn add_cluster_handle(&self) -> Result<AddClustersTransaction, Box<dyn Error>> {
        Ok(AddClustersTransaction {
            buffer: Vec::with_capacity(BUFFER_CAPACITY),
            db: &self.db,
        })
    }

    pub fn find_latest_cluster(
        &self,
        satellite: &str,
        sector: &str,
    ) -> Result<NaiveDateTime, Box<dyn Error>> {
        let latest: NaiveDateTime = self.db.query_row(
            include_str!("find_latest_cluster.sql"),
            &[satellite, sector],
            |row| row.get(0),
        )?;

        Ok(latest)
    }

    pub fn cluster_query_handle(&self) -> Result<ClusterQuery, Box<dyn Error>> {
        let stmt = self.db.prepare(include_str!("get_clusters.sql"))?;
        Ok(ClusterQuery(stmt))
    }
}

#[derive(Debug, Clone)]
pub struct ClusterRecord {
    // TODO add satellite as &'static str
    /// Row id from the database.
    pub rowid: i64,
    /// The start time of the scan this cluster was detected in.
    pub scan_time: NaiveDateTime,
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    pub power: f64,
    /// Perimeter
    pub perimeter: Polygon<f64>,
    /// Centroid
    pub centroid: Point<f64>,
}

//pub struct ClusterQuery<'a>(rusqlite::Statement<'a>);
pub struct ClusterQuery<'a>(rusqlite::Statement<'a>);

impl<'a> ClusterQuery<'a> {
    pub fn records_for(
        &mut self,
        satellite: &str,
    ) -> Result<impl Iterator<Item = ClusterRecord> + '_, Box<dyn Error>> {
        let rows = self
            .0
            .query_and_then(&[satellite], |row| {
                let rowid: i64 = row.get(0)?;
                let scan_time: NaiveDateTime =
                    chrono::NaiveDateTime::from_timestamp(row.get::<_, i64>(1)?, 0);
                let lat: f64 = row.get(2)?;
                let lon: f64 = row.get(3)?;
                let centroid = point!(x: lat, y: lon);
                let power: f64 = row.get(4)?;

                let pblob = row.get_ref(5)?.as_blob()?;

                let perimeter: Polygon<f64> =
                    bincode::deserialize(&pblob).map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(ClusterRecord {
                    rowid,
                    scan_time,
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
        &'static str,
        &'static str,
        NaiveDateTime,
        Point<f64>,
        f64,
        Polygon<f64>,
        i32,
    )>,
    db: &'a rusqlite::Connection,
}

const BUFFER_CAPACITY: usize = 100_000;

impl<'a> AddClustersTransaction<'a> {
    pub fn add_cluster(
        &mut self,
        satellite: &'static str,
        sector: &'static str,
        scan_start: NaiveDateTime,
        centroid: Point<f64>,
        power: f64,
        perimeter: Polygon<f64>,
        num_points: i32,
    ) -> Result<(), Box<dyn Error>> {
        self.buffer.push((
            satellite, sector, scan_start, centroid, power, perimeter, num_points,
        ));

        if self.buffer.len() >= BUFFER_CAPACITY {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        log::debug!("Flushing clusters.");
        self.db.execute_batch("BEGIN;")?;
        let mut stmt = self.db.prepare(include_str!("add_cluster.sql"))?;

        for (satellite, sector, scan_start, centroid, power, perimeter, num_points) in
            self.buffer.drain(..)
        {
            let lat = centroid.x();
            let lon = centroid.y();

            let perimeter = bincode::serialize(&perimeter)?;
            let _ = stmt.execute([
                &satellite as &dyn ToSql,
                &sector,
                &scan_start.timestamp(),
                &lat,
                &lon,
                &power,
                &num_points,
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
