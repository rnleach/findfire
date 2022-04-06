use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Parser;
use log::info;
use satfire::{BoundingBox, Coord, FiresDatabase, Geo, KmlFile, SatFireResult, Satellite};
use simple_logger::SimpleLogger;
use std::{
    fmt::{self, Display},
    path::PathBuf,
};
use strum::IntoEnumIterator;

/*-------------------------------------------------------------------------------------------------
 *                               Parse Command Line Arguments
 *-----------------------------------------------------------------------------------------------*/
///
/// Export fires into a KML file.
///
/// This program will export all the fires in a requested region and time range into a KML file.
///
#[derive(Debug, Parser)]
#[clap(bin_name = "showfires")]
#[clap(author, version, about)]
struct ShowFiresOptionsInit {
    /// The path to the fires database file.
    ///
    /// If this is not specified, then the program will check for it in the "FIRES_DB"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "FIRES_DB")]
    fires_store_file: PathBuf,

    /// The path to a KML file to produce from this run.
    ///
    /// If this is not specified, then the program will create one automatically by replacing the
    /// file extension on the fires_store_file with "*.kml".
    #[clap(short, long)]
    kml_file: Option<PathBuf>,

    /// The start time (UTC) for the export in the format YYYY-MM-DD-HH
    #[clap(parse(try_from_str=parse_datetime))]
    start: DateTime<Utc>,

    /// The end time (UTC) for the export in the format YYYY-MM-DD-HH
    #[clap(parse(try_from_str=parse_datetime))]
    end: DateTime<Utc>,

    /// Bounding Box where as bottom_lat,left_lon,top_lat,right_lon
    #[clap(parse(try_from_str=parse_bbox))]
    #[clap(default_value_t=BoundingBox{ll:Coord{lat: 44.0, lon: -116.5}, ur:Coord{lat: 49.5, lon: -104.0}})]
    bbox: BoundingBox,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

/// Parse a bounding box argument.
fn parse_bbox(bbox_str: &str) -> SatFireResult<BoundingBox> {
    let corners: Vec<_> = bbox_str.split(',').collect();

    if corners.len() < 4 {
        return Err("Invalid number of coords".into());
    }

    let min_lat = corners[0].parse()?;
    let min_lon = corners[1].parse()?;
    let max_lat = corners[2].parse()?;
    let max_lon = corners[3].parse()?;

    if min_lat >= max_lat || min_lon >= max_lon {
        return Err(format!(
            concat!(
                "Minimum Lat/Lon must be less than Maximum Lat/Lon:",
                " min_lat={} max_lat={} min_lon={} max_lon={}"
            ),
            min_lat, max_lat, min_lon, max_lon
        )
        .into());
    }

    if min_lat < -90.0 || max_lat > 90.0 || min_lon < -180.0 || max_lon > 180.0 {
        return Err(format!(
            concat!(
                "Lat/Lon are out of range (-90.0 to 90.0 and -180.0 to 180.0):",
                " min_lat={} max_lat={} min_lon={} max_lon={}"
            ),
            min_lat, max_lat, min_lon, max_lon
        )
        .into());
    }

    let ll = Coord {
        lat: min_lat,
        lon: min_lon,
    };
    let ur = Coord {
        lat: max_lat,
        lon: max_lon,
    };

    Ok(BoundingBox { ll, ur })
}

/// Parse a command line datetime
fn parse_datetime(dt_str: &str) -> SatFireResult<DateTime<Utc>> {
    const TIME_FORMAT: &str = "%Y-%m-%d-%H:%M:%S";
    let t_str = format!("{}:00:00", dt_str);

    let naive = NaiveDateTime::parse_from_str(&t_str, TIME_FORMAT)?;
    Ok(DateTime::from_utc(naive, Utc))
}

#[derive(Debug)]
struct ShowFiresOptionsChecked {
    /// The path to the database file.
    fires_store_file: PathBuf,

    /// The path to a KML file to produce from this run.
    kml_file: PathBuf,

    /// The start time.
    start: DateTime<Utc>,

    /// The end time.
    end: DateTime<Utc>,

    /// Verbose output
    verbose: bool,

    /// Bounding Box
    bbox: BoundingBox,
}

impl Display for ShowFiresOptionsChecked {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "\n")?; // yes, two blank lines.
        writeln!(f, "    Database: {}", self.fires_store_file.display())?;
        writeln!(f, "  Output KML: {}", self.kml_file.display())?;
        writeln!(f, "       Start: {}", self.start)?;
        writeln!(f, "         End: {}", self.end)?;
        writeln!(
            f,
            "Bounding Box: ({:.6}, {:.6}) <---> ({:.6}, {:.6})",
            self.bbox.ll.lat, self.bbox.ll.lon, self.bbox.ur.lat, self.bbox.ur.lon
        )?;
        writeln!(f, "\n")?; // yes, two blank lines.

        Ok(())
    }
}

/// Get the command line arguments and check them.
///
/// If there is missing data, try to fill it in with environment variables.
fn parse_args() -> SatFireResult<ShowFiresOptionsChecked> {
    let ShowFiresOptionsInit {
        fires_store_file,
        kml_file,
        start,
        end,
        bbox,
        verbose,
    } = ShowFiresOptionsInit::parse();

    let kml_file = match kml_file {
        Some(v) => v,
        None => {
            let mut clone = fires_store_file.clone();
            clone.set_extension("kml");
            clone
        }
    };

    let checked = ShowFiresOptionsChecked {
        fires_store_file,
        kml_file,
        start,
        end,
        bbox,
        verbose,
    };

    if verbose {
        info!("{}", checked);
    }

    Ok(checked)
}

/*-------------------------------------------------------------------------------------------------
 *                                             MAIN
 *-----------------------------------------------------------------------------------------------*/
fn main() -> SatFireResult<()> {
    SimpleLogger::new().init()?;

    let opts = parse_args()?;

    let db = FiresDatabase::connect(&opts.fires_store_file)?;
    let mut kfile = KmlFile::start_document(&opts.kml_file)?;

    kfile.start_style(Some("fire"))?;
    kfile.create_icon_style(
        Some("http://maps.google.com/mapfiles/kml/shapes/firedept.png"),
        0.5,
    )?;
    kfile.finish_style()?;

    let mut name = String::new();
    let mut description = String::new();
    let mut duration_buf = String::new();

    for sat in Satellite::iter() {
        kfile.start_folder(Some(sat.name()), None, false)?;

        let mut query = db.query_fires(Some(sat), opts.start, opts.end, opts.bbox)?;

        for fire_res in query.rows()? {
            match fire_res {
                Ok(fire) => {
                    let pixels = fire.pixels();
                    let Coord { lat, lon } = fire.centroid();

                    name.clear();
                    write!(&mut name as &mut dyn std::fmt::Write, "{}", fire.id())?;

                    kfile.start_folder(Some(&name), None, false)?;

                    kfile.timespan(fire.first_observed(), fire.last_observed())?;

                    fire.format_duration(&mut duration_buf);
                    description.clear();

                    write!(
                        &mut description as &mut dyn std::fmt::Write,
                        concat!(
                            "ID: {}<br/>",
                            "First Observed: {}<br/>",
                            "Last Observed: {}<br/>",
                            "Duration: {}<br/>",
                            "Max Power: {:.0} MW<br/>",
                            "Max Temperature: {:.0}K<br/>",
                        ),
                        fire.id(),
                        fire.first_observed(),
                        fire.last_observed(),
                        &duration_buf,
                        fire.max_power(),
                        fire.max_temperature(),
                    )?;

                    kfile.start_placemark(Some(&name), Some(&description), Some("#fire"))?;
                    kfile.create_point(lat, lon, 0.0)?;
                    kfile.finish_placemark()?;

                    pixels.kml_write(&mut kfile);

                    kfile.finish_folder()?;
                }
                Err(err) => {
                    if opts.verbose {
                        info!("Error reading fire from database: {}", err);
                    }
                }
            }
        }

        kfile.finish_folder()?;
    }

    Ok(())
}
