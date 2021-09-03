use std::error::Error;

use log::LevelFilter;
use simple_logger::SimpleLogger;
use satfire::ClusterDatabase;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";

fn main() -> Result<(), Box<dyn Error>> {

    SimpleLogger::new()
        .with_module_level("goes_arch", LevelFilter::Debug)
        .with_module_level("serde_xml_rs", LevelFilter::Off)
        .with_module_level("reqwest", LevelFilter::Off)
        .init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let _cluster_db = ClusterDatabase::connect(DATABASE_FILE)?;

    Ok(())
}
