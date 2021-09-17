/*! Methods and types to support querying the clusters table of the database. */

use crate::FireCode;
use chrono::NaiveDateTime;
use geo::MultiPolygon;
use rusqlite::{Connection, ToSql};
use std::error::Error;

const BUFFER_SIZE: usize = 1_000;

impl super::FiresDatabase {
    pub fn add_association_handle(&self) -> Result<AddAssociationsTransaction, Box<dyn Error>> {
        Ok(AddAssociationsTransaction {
            buffer: Vec::with_capacity(BUFFER_SIZE),
            db: &self.db,
        })
    }
}

pub struct AddAssociationsTransaction<'a> {
    buffer: Vec<(FireCode, NaiveDateTime, f64, MultiPolygon<f64>)>,
    db: &'a Connection,
}

impl<'a> AddAssociationsTransaction<'a> {
    pub fn add_association(
        &mut self,
        fire_id: FireCode,
        scan_time: NaiveDateTime,
        power: f64,
        perimeter: MultiPolygon<f64>,
    ) -> Result<(), Box<dyn Error>> {
        self.buffer.push((fire_id, scan_time, power, perimeter));

        if self.buffer.len() >= BUFFER_SIZE {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        log::trace!("Flushing associations.");
        let mut stmt = self.db.prepare(include_str!("add_association.sql"))?;

        self.db.execute_batch("BEGIN;")?;

        for (fire_id, scan_time, power, perimeter) in self.buffer.drain(..) {
            let perimeter = bincode::serialize(&perimeter)?;

            let _ = stmt.execute([
                &fire_id.as_ref() as &dyn ToSql,
                &scan_time.timestamp(),
                &power,
                &perimeter,
            ])?;
        }

        self.db.execute_batch("COMMIT;")?;
        log::trace!("Flushed associations.");
        Ok(())
    }
}

impl<'a> Drop for AddAssociationsTransaction<'a> {
    fn drop(&mut self) {
        log::debug!("Dropping AddAssociationsTransaction");
        self.flush().unwrap()
    }
}
