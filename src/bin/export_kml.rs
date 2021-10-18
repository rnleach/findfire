use chrono::NaiveDate;
use kml::{
    types::{Element, Geometry, MultiGeometry, Placemark, PolyStyle, Style},
    Kml, KmlWriter,
};
use log::LevelFilter;
use satfire::{FiresDatabase, Satellite};
use simple_logger::SimpleLogger;
use std::{collections::HashMap, error::Error, fs::File};

const FIRES_DATABASE_FILE: &str = "/home/ryan/wxdata/connectfire.sqlite";
const OUTPUT_FILE_G16: &str = "/home/ryan/wxdata/connectfire_g16.kml";
const OUTPUT_FILE_G17: &str = "/home/ryan/wxdata/connectfire_g17.kml";

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().with_level(LevelFilter::Debug).init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let start = NaiveDate::from_ymd(2021, 1, 1).and_hms(0, 0, 0);
    let end = NaiveDate::from_ymd(2022, 1, 1).and_hms(0, 0, 0);

    log::info!("start -> end : {} -> {}", start, end);

    let fdb = FiresDatabase::connect(FIRES_DATABASE_FILE)?;
    let mut fires = fdb.read_fires_handle()?;
    for (sat, output_file) in [
        (Satellite::G16, OUTPUT_FILE_G16),
        (Satellite::G17, OUTPUT_FILE_G17),
    ] {
        let fires = fires
            .records_for(sat)?
            .filter(|fr| fr.last_observed > start)
            .filter(|fr| fr.last_observed < end);

        let mut elements = vec![];

        let poly_style = match sat {
            Satellite::G16 => PolyStyle {
                id: "fire_poly_g16".to_owned(),
                color: "7F0000FF".to_owned(),
                fill: true,
                outline: false,
                color_mode: kml::types::ColorMode::Default,
            },
            Satellite::G17 => PolyStyle {
                id: "fire_poly_g17".to_owned(),
                color: "7F00FFFF".to_owned(),
                fill: true,
                outline: false,
                color_mode: kml::types::ColorMode::Default,
            },
        };

        let poly_style = Kml::Style(Style {
            id: "fire".to_owned(),
            poly: Some(poly_style),
            ..Style::default()
        });

        for (i, fr) in fires.enumerate() {
            let geometry = Some(Geometry::MultiGeometry(MultiGeometry::from(fr.perimeter)));
            let name = Some(fr.id.clone_string());
            let description = Some(format!("Last observed: {}", fr.last_observed));
            let placemark = Placemark {
                geometry,
                name,
                description,
                children: vec![Element {
                    name: "styleUrl".into(),
                    content: Some("#fire".into()),
                    ..Element::default()
                }],
                ..Placemark::default()
            };

            elements.push(Kml::Placemark(placemark));

            if i % 100 == 0 {
                log::debug!("Up to {}", i);
            }
        }

        let folder_name: Kml<f64> = Kml::Element(Element {
            name: "name".into(),
            content: Some("GOES Fire Detections".into()),
            ..Element::default()
        });

        elements.push(folder_name);

        let folder = Kml::Folder {
            elements,
            attrs: HashMap::new(),
        };

        let doc = Kml::Document {
            elements: vec![folder, poly_style],
            attrs: HashMap::new(),
        };

        let mut f = File::create(output_file)?;
        let mut output = KmlWriter::from_writer(&mut f);

        output.write(&doc)?;
    }

    Ok(())
}
