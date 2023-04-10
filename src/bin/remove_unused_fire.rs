//! Documentation for the binary is with the definition of `RemoveUnusedOptionsInit` below.

use clap::Parser;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, info, trace, warn, LevelFilter};
use satfire::{ClusterDatabase, SatFireResult, Satellite, Sector};
use simple_logger::SimpleLogger;
use std::{
    path::{Path, PathBuf},
    thread::JoinHandle,
};

/*-------------------------------------------------------------------------------------------------
 *                               Parse Command Line Arguments
 *-----------------------------------------------------------------------------------------------*/
///
/// Search for files with that had no clusters analyzed, and remove them.
///
#[derive(Debug, Parser)]
#[clap(bin_name = "remove_unused_fire")]
#[clap(author, version, about)]
struct RemoveUnusedOptionsInit {
    /// The path to the cluster database file.
    ///
    /// If this is not specified, then the program will check for it in the "CLUSTER_DB"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "CLUSTER_DB")]
    cluster_store_file: PathBuf,

    /// The path to the data directory that will be walked to find unused files.
    ///
    /// If this is not specified, then the program will check for it in the "SAT_ARCHIVE"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "SAT_ARCHIVE")]
    data_dir: PathBuf,

    /// Default to a dry run, but if execute, then actually delete the files.
    #[clap(short, long)]
    execute: bool,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

#[derive(Debug)]
struct RemoveUnusedOptionsChecked {
    /// The path to the database file.
    cluster_store_file: PathBuf,

    /// The path to the data directory that will be walked to find new data.
    data_dir: PathBuf,

    /// Verbose output
    verbose: bool,

    /// Default to a dry run, but if execute, then actually delete the files.
    execute: bool,
}

/// Get the command line arguments and check them.
///
/// If there is missing data, try to fill it in with environment variables.
fn parse_args() -> SatFireResult<RemoveUnusedOptionsChecked> {
    let RemoveUnusedOptionsInit {
        cluster_store_file,
        data_dir,
        execute,
        verbose,
    } = RemoveUnusedOptionsInit::parse();

    Ok(RemoveUnusedOptionsChecked {
        cluster_store_file,
        data_dir,
        verbose,
        execute,
    })
}

/*-------------------------------------------------------------------------------------------------
 *                                            Main
 *-----------------------------------------------------------------------------------------------*/
fn main() -> SatFireResult<()> {
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;

    let opts = parse_args()?;

    if opts.verbose {
        info!(target: "startup", "{:#?}", opts);
        debug!(target: "startup", "Debug logging active.");
    }

    ClusterDatabase::initialize(&opts.cluster_store_file)?;

    let (to_no_fire_filter, from_dir_walker) = bounded(128);
    let (to_deleter, from_no_fire_filter) = bounded(128);

    let data_dir = &opts.data_dir;
    let store_file = &opts.cluster_store_file;
    let verbose = opts.verbose;
    let execute = opts.execute;

    let walk_dir = dir_walker(data_dir, to_no_fire_filter)?;
    let no_fire = filter_no_fire(store_file, from_dir_walker, to_deleter, verbose)?;
    let deleter = deleter_thread(from_no_fire_filter, execute, verbose)?;

    walk_dir.join().expect("Error joining dir walker thread")?;

    for jh in no_fire {
        jh.join().expect("Error joining filter thread")?;
    }

    let num_deleted = deleter.join().expect("Error joining deleter thread")?;

    if verbose {
        if execute {
            info!(target: "summary", "Deleted {} files.", num_deleted);
        } else {
            info!(target: "summary", "Can delete {} files.", num_deleted);
        }
    }

    Ok(())
}

/*-------------------------------------------------------------------------------------------------
 *                           Threads - Functions that start threads
 *-----------------------------------------------------------------------------------------------*/
fn dir_walker<P: AsRef<Path>>(
    data_dir: P,
    to_no_fire_filter: Sender<PathBuf>,
) -> SatFireResult<JoinHandle<SatFireResult<()>>> {
    let data_dir = data_dir.as_ref().to_path_buf();

    let standard_path_filter = create_standard_path_filter();

    let jh = std::thread::Builder::new()
        .name("no-fire-walker".to_owned())
        .spawn(move || {
            for entry in walkdir::WalkDir::new(data_dir)
                .into_iter()
                .filter_entry(standard_path_filter)
                // Skip errors silently
                .filter_map(|res| res.ok())
            {
                if entry.depth() <= 4 {
                    info!(target: "dir-walker", "Checking: {}", entry.path().display());
                } else {
                    debug!(target: "dir-walker", "Checking: {}", entry.path().display());
                }
                to_no_fire_filter.send(entry.into_path())?;
            }

            Ok(())
        })?;

    Ok(jh)
}

fn filter_no_fire<P: AsRef<Path>>(
    store_file: P,
    from_dir_walker: Receiver<PathBuf>,
    to_deleter: Sender<PathBuf>,
    verbose: bool,
) -> SatFireResult<Vec<JoinHandle<SatFireResult<()>>>> {
    let store_file = store_file.as_ref().to_path_buf();

    let mut handles = Vec::with_capacity(num_cpus::get());

    for _ in 0..num_cpus::get() {
        let to_deleter_clone = to_deleter.clone();
        let from_dir_walker_clone = from_dir_walker.clone();
        let store_file_clone = store_file.clone();

        let jh = std::thread::Builder::new()
            .name("no-fire-filter".to_owned())
            .spawn(move || {
                let db = ClusterDatabase::connect(store_file_clone)?;
                let mut no_fire = db.prepare_to_query_clusters_present()?;

                for path in from_dir_walker_clone {
                    if let Some((sat, sector, start, end)) = path.file_name().and_then(|fname| {
                        satfire::parse_satellite_description_from_file_name(&fname.to_string_lossy())
                    }) {
                        if no_fire.present_no_fire(sat, sector, start, end)? {
                            if verbose {
                                debug!(target: "filter", "can remove: {} {} {} - {}", sat, sector, start, path.display());
                            }

                            to_deleter_clone.send(path)?;
                        } else if verbose {
                            trace!(target: "filter", "contains data or not processed: {}", path.display());
                        }
                    }
                }
                Ok(())
            })?;

        handles.push(jh);
    }

    Ok(handles)
}

fn deleter_thread(
    from_no_fire_filter: Receiver<PathBuf>,
    execute: bool,
    verbose: bool,
) -> SatFireResult<JoinHandle<SatFireResult<u64>>> {
    let jh = std::thread::Builder::new()
        .name("no-fire-del".to_owned())
        .spawn(move || {
            let mut count: u64 = 0;

            for path in from_no_fire_filter {
                if verbose {
                    info!(target: "delete", "Removing {}", path.display());
                }

                count += 1;

                if execute {
                    match std::fs::remove_file(&path) {
                        Ok(_) => {}
                        Err(e) => {
                            warn!(target: "delete", "Error deleting {} :: {}", path.display(), e)
                        }
                    }
                }
            }

            Ok(count)
        })?;

    Ok(jh)
}

/*-------------------------------------------------------------------------------------------------
 *                         Filters for skipping files / directories / clusters
 *-----------------------------------------------------------------------------------------------*/
fn create_standard_path_filter() -> impl FnMut(&walkdir::DirEntry) -> bool {
    /* This filter assumes the data is stored in a directory tree like:
     *   SATELLITE/SECTOR/YEAR/DAY_OF_YEAR/HOUR/files
     *
     *   e.g.
     *   G16/ABI-L2-FDCF/2020/238/15/...files...
     */

    move |entry| -> bool {
        if entry.path().is_file() {
            // Keep files with the proper extension.
            let keep = entry
                .path()
                .extension()
                .map(|ex| ex == "nc")
                .unwrap_or(false)
                || entry
                    .path()
                    .extension()
                    .map(|ex| ex == "zip")
                    .unwrap_or(false);

            //debug!(target: "path filter", "keep: {} path: {}", keep, entry.path().display());
            keep
        } else if entry.path().is_dir() {
            // Let's trim directories we KNOW have data that is too old
            let path = entry.path().to_string_lossy();

            // Get the satellite and sector. If we can't parse these, then we need to keep going
            // deeper.
            let _sat = match Satellite::string_contains_satellite(&path) {
                Some(sat) => sat,
                None => return true,
            };

            let _sector = match Sector::string_contains_sector(&path) {
                Some(sector) => sector,
                None => return true,
            };

            // Not enough info, keep going!
            true
        } else {
            // If we can't tell, accept it for now
            true
        }
    }
}
