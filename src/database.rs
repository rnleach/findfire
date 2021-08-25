use std::{error::Error, path::Path};

use chrono::NaiveDateTime;
use rusqlite::ToSql;

pub struct ClusterDatabase {
    db: rusqlite::Connection,
}

impl ClusterDatabase {
    pub fn connect<P: AsRef<Path>>(path_to_db: P) -> Result<Self, Box<dyn Error>> {

        let conn = rusqlite::Connection::open_with_flags(
            path_to_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        conn.execute(include_str!("create_db.sql"), [])?;

        Ok(ClusterDatabase { db: conn })
    }

    pub fn prepare(&self) -> Result<AddRowsTransaction, Box<dyn Error>> {
        self.db.execute("BEGIN", [])?;

        let stmt = self.db.prepare(include_str!("add_row_statement.sql"))?;
        Ok(AddRowsTransaction(stmt, &self.db))
    }
}

pub struct AddRowsTransaction<'a>(rusqlite::Statement<'a>, &'a rusqlite::Connection);

impl<'a> AddRowsTransaction<'a> {
    pub fn add_row(
        &mut self,
        satellite: &'static str,
        sector: &'static str,
        scan_start: NaiveDateTime,
        lat: f64,
        lon: f64,
        power: f64,
        radius: f64,
        num_points: i32,
    ) -> Result<(), Box<dyn Error>> {
        let _ = self.0.query([
            &satellite as &dyn ToSql,
            &sector,
            &scan_start,
            &lat,
            &lon,
            &power,
            &radius,
            &num_points,
        ])?;

        Ok(())
    }
}

impl<'a> Drop for AddRowsTransaction<'a> {
    fn drop(&mut self) {
        self.1.execute("COMMIT", []).unwrap();
    }
}
