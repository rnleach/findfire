use clap::Parser;
use log::info;
use satfire::{
    ClusterDatabaseClusterRow, Coord, JointFiresClusterDatabases, KmlWriter, KmzFile, SatFireResult,
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

    let mut name = String::new();
    let mut description = String::new();
    for cluster in query.rows()? {
        let cluster = cluster?;

        let ClusterDatabaseClusterRow {
            rowid,
            sat,
            sector,
            power,
            scan_angle,
            max_temperature,
            area,
            centroid: Coord { lat, lon },
            pixels,
            start,
            end,
        } = cluster;

        name.clear();
        let _ = write!(&mut name, "{}", rowid);

        description.clear();
        let _ = write!(
            &mut description,
            concat!(
                "<h3>Cluster Power: {:.0}MW</h3>",
                "<h3>Max Scan Angle: {:.2}&deg;</h3>",
                "<h3>Max Temperature: {:.2}&deg;K</h3>",
                "<h3>Area: {:.0}m&sup2;</h3>",
                "<h3>Satellite: {}</h3>",
                "<h3>Scan Sector: {}</h3>",
            ),
            power,
            scan_angle,
            max_temperature,
            area,
            sat.name(),
            sector.name(),
        );

        kfile.start_folder(Some(&name), None, false)?;
        kfile.timespan(start, end)?;

        kfile.start_placemark(None, Some(&description), Some("#fire"))?;
        kfile.create_point(lat, lon, 0.0)?;
        kfile.finish_placemark()?;
        pixels.kml_write(&mut kfile);
        kfile.finish_folder()?;
    }

    Ok(())
}
