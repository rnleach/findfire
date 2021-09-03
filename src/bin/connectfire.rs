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

    let _cluster_db = ClusterDatabase::connect(DATABASE_FILE)?;

    Ok(())
}
