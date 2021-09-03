use std::error::Error;

use satfire::ClusterDatabase;
use simple_logger::SimpleLogger;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let cluster_db = ClusterDatabase::connect(DATABASE_FILE)?;
    let mut records = cluster_db.create_cluster_record_query()?;
    let records = records.cluster_records_for("G17")?;

    for record in records {
        println!("{:?}", record);
    }

    Ok(())
}
