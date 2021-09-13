/*! Methods and types to support querying the clusters table of the database. */

use rusqlite::{Connection, ToSql};
use std::error::Error;

const BUFFER_SIZE: usize = 100_000;

impl super::FiresDatabase {
    pub fn add_association_handle(&self) -> Result<AddAssociationsTransaction, Box<dyn Error>> {
        Ok(AddAssociationsTransaction {
            buffer: Vec::with_capacity(BUFFER_SIZE),
            db: &self.db,
        })
    }
}

pub struct AddAssociationsTransaction<'a> {
    buffer: Vec<(i64, String)>,
    db: &'a Connection,
}

impl<'a> AddAssociationsTransaction<'a> {
    pub fn add_association<S: Into<String>>(
        &mut self,
        rowid: i64,
        fire_id: S,
    ) -> Result<(), Box<dyn Error>> {
        let fire_id: String = fire_id.into();

        self.buffer.push((rowid, fire_id));

        if self.buffer.len() >= BUFFER_SIZE {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        log::debug!("Flushing associations.");
        let mut stmt = self
            .db
            .prepare("INSERT INTO associations (cluster_row_id, fire_id) VALUES (?, ?)")?;

        self.db.execute_batch("BEGIN;")?;

        for (rowid, fire_id) in self.buffer.drain(..) {
            let _ = stmt.execute([&rowid as &dyn ToSql, &fire_id])?;
        }

        self.db.execute_batch("COMMIT;")?;
        log::debug!("Flushed associations.");
        Ok(())
    }
}

impl<'a> Drop for AddAssociationsTransaction<'a> {
    fn drop(&mut self) {
        log::debug!("Dropping AddAssociationsTransaction");
        self.flush().unwrap()
    }
}
