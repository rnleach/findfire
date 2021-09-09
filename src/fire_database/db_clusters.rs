/*! Methods and types to support querying the clusters table of the database. */

use std::error::Error;

use crate::ClusterRecord;
use chrono::NaiveDateTime;
use geo::{point, Point, Polygon};
use rusqlite::ToSql;

impl super::FiresDatabase {
    pub fn add_cluster_handle(&self) -> Result<AddClustersTransaction, Box<dyn Error>> {
        let stmt = self.db.prepare(include_str!("add_cluster.sql"))?;

        self.db.execute("BEGIN", [])?;
        Ok(AddClustersTransaction(stmt, &self.db))
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

pub struct ClusterQuery<'a>(rusqlite::Statement<'a>);

impl<'a> ClusterQuery<'a> {
    pub fn records_for(
        &mut self,
        satellite: &str,
    ) -> Result<impl Iterator<Item = ClusterRecord> + '_, Box<dyn Error>> {
        let rows = self
            .0
            .query_and_then(&[satellite], |row| {
                let id: i64 = row.get(0)?;
                let valid_time: NaiveDateTime =
                    chrono::NaiveDateTime::from_timestamp(row.get::<_, i64>(1)?, 0);
                let lat: f64 = row.get(2)?;
                let lon: f64 = row.get(3)?;
                let centroid = point!(x: lat, y: lon);
                let power: f64 = row.get(4)?;

                let pblob = row.get_ref(5)?.as_blob()?;

                let perimeter: Polygon<f64> =
                    bincode::deserialize(&pblob).map_err(|_| rusqlite::Error::InvalidQuery)?;

                Ok(ClusterRecord::new(
                    id, valid_time, power, perimeter, centroid,
                ))
            })?
            .filter_map(|res: Result<_, rusqlite::Error>| res.ok());

        Ok(rows)
    }
}

pub struct AddClustersTransaction<'a>(rusqlite::Statement<'a>, &'a rusqlite::Connection);

impl<'a> AddClustersTransaction<'a> {
    pub fn add_cluster(
        &mut self,
        satellite: &'static str,
        sector: &'static str,
        scan_mid_point: NaiveDateTime,
        centroid: Point<f64>,
        power: f64,
        perimeter: Polygon<f64>,
        num_points: i32,
    ) -> Result<(), Box<dyn Error>> {
        let lat = centroid.x();
        let lon = centroid.y();

        let perimeter = bincode::serialize(&perimeter)?;

        let _ = self.0.execute([
            &satellite as &dyn ToSql,
            &sector,
            &scan_mid_point.timestamp(),
            &lat,
            &lon,
            &power,
            &num_points,
            &perimeter,
        ])?;

        Ok(())
    }
}

impl<'a> Drop for AddClustersTransaction<'a> {
    fn drop(&mut self) {
        self.1.execute("COMMIT", []).unwrap();
    }
}
