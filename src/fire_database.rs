use std::{error::Error, path::Path};

pub struct FiresDatabase {
    db: rusqlite::Connection,
}

mod db_clusters;
pub use db_clusters::{AddClustersTransaction, ClusterQuery, ClusterRecord};
mod db_fires;
pub use db_fires::AddFireTransaction;

impl FiresDatabase {
    pub fn connect<P: AsRef<Path>>(path_to_db: P) -> Result<Self, Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            path_to_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        conn.execute_batch(include_str!("fire_database/create_db.sql"))?;

        Ok(FiresDatabase { db: conn })
    }
}
