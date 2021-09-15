use std::{error::Error, path::Path};

pub struct FiresDatabase {
    db: rusqlite::Connection,
}

mod db_fires;
pub use db_fires::{AddFireTransaction, FireCode, FireDataNextNewFireState, FireQuery, FireRecord};
mod db_cluster_fire_associations;
pub use db_cluster_fire_associations::AddAssociationsTransaction;

impl FiresDatabase {
    pub fn connect<P: AsRef<Path>>(path_to_db: P) -> Result<Self, Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            path_to_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        conn.execute_batch(include_str!("fire_database/create_fires_db.sql"))?;

        Ok(FiresDatabase { db: conn })
    }
}
