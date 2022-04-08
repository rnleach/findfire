use clap::Parser;
use chrono::Duration;
use log::info;
use satfire::{FireList, FiresDatabase, SatFireResult, Satellite};
use simple_logger::SimpleLogger;
use std::{
    cmp::Reverse,
    fmt::{self, Display},
    path::PathBuf,
};

/*-------------------------------------------------------------------------------------------------
 *                                     Command Line Options
 *-----------------------------------------------------------------------------------------------*/

///
/// Export active fires into a KML file.
///
/// This program will export all the fires not deemed as stale in the database for a given
/// satellite as KML.
///
#[derive(Debug, Parser)]
#[clap(bin_name = "currentfires")]
#[clap(author, version, about)]
struct CurrentFiresOptionsInit {
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
    /// file extension on the store_file with "*.kml".
    #[clap(short, long)]
    kml_file: Option<PathBuf>,

    /// The satellite to export the data for.
    ///
    /// If this is not specified, then it will default to GOES-17. Allowed values are G16 and G17.
    #[clap(parse(try_from_str=parse_satellite))]
    #[clap(default_value_t=Satellite::G17)]
    sat: Satellite,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

fn parse_satellite(sat: &str) -> SatFireResult<Satellite> {
    let sat = Satellite::string_contains_satellite(sat)
        .ok_or_else(|| format!("Argument is not a valid satellite name: {}", sat))?;
    Ok(sat)
}

#[derive(Debug)]
struct CurrentFiresOptionsChecked {
    /// The path to the database file.
    fires_store_file: PathBuf,

    /// The path to a KML file to produce from this run.
    kml_file: PathBuf,

    /// The satellite.
    sat: Satellite,

    /// Verbose output
    verbose: bool,
}

impl Display for CurrentFiresOptionsChecked {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "\n")?; // yes, two blank lines.
        writeln!(f, "    Database: {}", self.fires_store_file.display())?;
        writeln!(f, "  Output KML: {}", self.kml_file.display())?;
        writeln!(f, "   Satellite: {}", self.sat.name())?;
        writeln!(f, "\n")?; // yes, two blank lines.

        Ok(())
    }
}

/// Get the command line arguments and check them.
///
/// If there is missing data, try to fill it in with environment variables.
fn parse_args() -> SatFireResult<CurrentFiresOptionsChecked> {
    let CurrentFiresOptionsInit {
        fires_store_file,
        kml_file,
        sat,
        verbose,
    } = CurrentFiresOptionsInit::parse();

    let kml_file = match kml_file {
        Some(v) => v,
        None => {
            let mut clone = fires_store_file.clone();
            clone.set_extension("kml");
            clone
        }
    };

    let checked = CurrentFiresOptionsChecked {
        fires_store_file,
        kml_file,
        sat,
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

    //
    // Load the data, the most recent clusters.
    //
    let db = FiresDatabase::connect(&opts.fires_store_file)?;
    let active_fires = db.ongoing_fires(opts.sat)?;
    drop(db);

    let mut active_fires = active_fires.into_vec();
    active_fires.sort_unstable_by_key(|a| Reverse(a.duration()));
    let active_fires = FireList::from(active_fires);

    if opts.verbose {
        info!("Retrieved {} fires.", active_fires.len());
    }

    active_fires.save_kml(Duration::days(1), &opts.kml_file)?;

    Ok(())
}
