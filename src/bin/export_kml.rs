use chrono::NaiveDate;
use kml::{
    types::{Geometry, Placemark, Polygon},
    Kml, KmlWriter,
};
use log::LevelFilter;
use satfire::{FiresDatabase, Satellite};
use simple_logger::SimpleLogger;
use std::{collections::HashMap, error::Error, fs::File};

const FIRES_DATABASE_FILE: &'static str = "/Users/ryan/wxdata/connectfire.sqlite";
const OUTPUT_FILE: &'static str = "/Users/ryan/wxdata/connectfire.kml";

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().with_level(LevelFilter::Debug).init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let start = NaiveDate::from_ymd(2018, 1, 1).and_hms(0, 0, 0);
    let end = NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0);

    log::info!("start -> end : {} -> {}", start, end);

    let fdb = FiresDatabase::connect(FIRES_DATABASE_FILE)?;
    let mut fires = fdb.read_fires_handle()?;
    let fires = fires
        .records_for(Satellite::G16)?
        .filter(|fr| fr.last_observed > start)
        .filter(|fr| fr.last_observed < end);

    let mut elements = vec![];

    for (i, fr) in fires.enumerate() {
        let geometry = Some(Geometry::Polygon(Polygon::from(fr.perimeter)));
        let name = Some(fr.id.clone_string());
        let description = Some(format!("last_observed: {}", fr.last_observed));
        let placemark = Placemark {
            geometry,
            name,
            description,
            ..Placemark::default()
        };

        elements.push(Kml::Placemark(placemark));

        if i % 100 == 0 {
            log::debug!("Up to {}", i);
        }
    }

    let doc = Kml::Document {
        elements,
        attrs: HashMap::new(),
    };

    let mut f = File::create(OUTPUT_FILE)?;
    let mut output = KmlWriter::from_writer(&mut f);

    output.write(&doc)?;

    Ok(())
}
