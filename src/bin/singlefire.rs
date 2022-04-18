use clap::Parser;
use log::info;
use satfire::{
    Coord, Geo, JointFiresClusterDatabases, KmlWriter, KmzFile,
    SatFireResult,
};
use simple_logger::SimpleLogger;
use std::{
    fmt::{self, Display, Write},
    path::PathBuf,
};

/*-------------------------------------------------------------------------------------------------
 *                                     Command Line Options
 *-----------------------------------------------------------------------------------------------*/

/// Select a single fire based on its identification fire_id value (perhaps by viewing output from
/// showfires) and output all clusters that eventually contributed to that fire in a KMZ file.
#[derive(Debug, Parser)]
#[clap(bin_name = "singlefire")]
#[clap(author, version, about)]
struct SingleFireOptions {
    /// The fire_id of the fire to export in a KMZ
    fire_id: u64,

    /// The path to a KMZ file to produce from this fire.
    kmz_file: PathBuf,

    /// The path to the database file with the clusters.
    ///
    /// If this is not specified, then the program will check for it in the "CLUSTER_DB"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "CLUSTER_DB")]
    clusters_store_file: PathBuf,

    /// The path to the database file with the fires and associations.
    ///
    /// If this is not specified, then the program will check for it in the "FIRES_DB"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "FIRES_DB")]
    fires_store_file: PathBuf,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

impl Display for SingleFireOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "\n")?; // yes, two blank lines.
        writeln!(
            f,
            "Cluster Database: {}",
            self.clusters_store_file.display()
        )?;
        writeln!(f, "  Fires Database: {}", self.fires_store_file.display())?;
        writeln!(f, "\n")?; // yes, two blank lines.

        Ok(())
    }
}

/// Get the command line arguments and check them.
///
/// If there is missing data, try to fill it in with environment variables.
fn parse_args() -> SatFireResult<SingleFireOptions> {
    let opts = SingleFireOptions::parse();

    if opts.verbose {
        info!(target:"startup", "{}", opts);
    }

    Ok(opts)
}

/*-------------------------------------------------------------------------------------------------
 *                                             Main
 *-----------------------------------------------------------------------------------------------*/
fn main() -> SatFireResult<()> {
    SimpleLogger::new().init()?;

    let opts = parse_args()?;

    let dbs =
        JointFiresClusterDatabases::connect(&opts.clusters_store_file, &opts.fires_store_file)?;

    let mut query = dbs.single_fire_query(opts.fire_id)?;

    //
    // Output the KMZ
    //
    let mut kfile = KmzFile::new(&opts.kmz_file)?;

    kfile.start_style(Some("fire"))?;
    kfile.create_icon_style(None, 0.0)?;
    kfile.finish_style()?;

    let mut description = String::new();
    let mut hour_of_data = Vec::new();
    let mut current_hour_ts = 0;

    for group in query.rows()?.filter_map(Result::ok).filter_map(|cluster| {
        // Works because satellite times are all after 1970
        let mut cluster_hour_ts = cluster.start.timestamp();
        cluster_hour_ts -= cluster_hour_ts % 3_600;

        if current_hour_ts != cluster_hour_ts {
            let mut to_ret = Vec::with_capacity(hour_of_data.len());
            std::mem::swap(&mut to_ret, &mut hour_of_data);

            hour_of_data.push(cluster);
            current_hour_ts = cluster_hour_ts;
            Some(to_ret)
        } else {
            hour_of_data.push(cluster);
            None
        }
    }) {
        if !group.is_empty() {
            let start = group[0].start;
            let end = group.iter().last().map(|c| c.end).unwrap_or(start);

            let mut max_power: f64 = -f64::INFINITY;
            let mut max_temp: f64 = -f64::INFINITY;
            let mut pixels = satfire::PixelList::new();

            for cluster in group {
                pixels.max_merge(&cluster.pixels);
                max_power = max_power.max(cluster.power);
                max_temp = max_temp.max(cluster.max_temperature);
            }

            description.clear();
            let _ = write!(
                &mut description,
                concat!(
                    "<h3>Cluster Power: {:.0}MW</h3>",
                    "<h3>Max Temperature: {:.2}&deg;K</h3>",
                ),
                max_power, max_temp,
            );

            let Coord { lat, lon } = pixels.centroid();

            kfile.start_folder(None, None, false)?;
            kfile.timespan(start, end)?;

            kfile.start_placemark(None, Some(&description), Some("#fire"))?;
            kfile.create_point(lat, lon, 0.0)?;
            kfile.finish_placemark()?;
            pixels.kml_write(&mut kfile);
            kfile.finish_folder()?;
        }
    }

    Ok(())
}
