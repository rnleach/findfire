use std::{error::Error, path::Path};

use crate::ClusterRecord;
use chrono::NaiveDateTime;
use geo::{point, Point, Polygon};
use rusqlite::ToSql;

pub struct FiresDatabase {
    db: rusqlite::Connection,
}

impl FiresDatabase {
    pub fn connect<P: AsRef<Path>>(path_to_db: P) -> Result<Self, Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            path_to_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        conn.execute(include_str!("create_db.sql"), [])?;

        Ok(FiresDatabase { db: conn })
    }

    pub fn prepare(&self) -> Result<AddRowsTransaction, Box<dyn Error>> {
        let stmt = self.db.prepare(include_str!("add_row_statement.sql"))?;

        self.db.execute("BEGIN", [])?;
        Ok(AddRowsTransaction(stmt, &self.db))
    }

    pub fn find_latest(
        &self,
        satellite: &str,
        sector: &str,
    ) -> Result<NaiveDateTime, Box<dyn Error>> {
        let latest: NaiveDateTime = self.db.query_row(
            include_str!("find_latest.sql"),
            &[satellite, sector],
            |row| row.get(0),
        )?;

        Ok(latest)
    }

    pub fn create_cluster_record_query(&self) -> Result<ClusterRecordQuery, Box<dyn Error>> {
        let stmt = self.db.prepare("SELECT rowid, mid_point_time, lat, lon, power, perimeter FROM clusters WHERE satellite = ? ORDER BY mid_point_time ASC")?;
        Ok(ClusterRecordQuery(stmt))
    }
}

pub struct ClusterRecordQuery<'a>(rusqlite::Statement<'a>);

impl<'a> ClusterRecordQuery<'a> {
    pub fn cluster_records_for(
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

pub struct AddRowsTransaction<'a>(rusqlite::Statement<'a>, &'a rusqlite::Connection);

impl<'a> AddRowsTransaction<'a> {
    pub fn add_row(
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

impl<'a> Drop for AddRowsTransaction<'a> {
    fn drop(&mut self) {
        self.1.execute("COMMIT", []).unwrap();
    }
}
