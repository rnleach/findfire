use std::{error::Error, path::Path};

pub struct ClustersDatabase {
    db: rusqlite::Connection,
}

mod db_clusters;
pub use db_clusters::{AddClustersTransaction, ClusterQuery, ClusterRecord};

impl ClustersDatabase {
    pub fn connect<P: AsRef<Path>>(path_to_db: P) -> Result<Self, Box<dyn Error>> {
        let conn = rusqlite::Connection::open_with_flags(
            path_to_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        conn.execute_batch(include_str!("cluster_database/create_cluster_db.sql"))?;

        Ok(ClustersDatabase { db: conn })
    }
}
