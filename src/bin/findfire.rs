use std::{
    collections::HashMap,
    error::Error,
    thread::{self},
};

use chrono::NaiveDateTime;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::LevelFilter;
use satfire::{Cluster, ClustersDatabase, FireSatImage, Satellite, Sector};
use simple_logger::SimpleLogger;
use strum::IntoEnumIterator;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";
const DATA_DIR: &'static str = "/media/ryan/SAT/GOESX/";

const CHANNEL_SIZE: usize = 100;

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().with_level(LevelFilter::Info).init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let (to_load_thread, from_path_gen) = bounded(CHANNEL_SIZE);
    let (to_analysis, from_load_thread) = bounded(CHANNEL_SIZE);
    let (to_database_thread, from_analysis_thread) = bounded(CHANNEL_SIZE);

    let path_gen_jh = thread::Builder::new()
        .name("path_gen-findfire".to_owned())
        .spawn(move || {
            generate_paths(to_load_thread);
        })?;

    let load_jh = thread::Builder::new()
        .name("load-findfire".to_owned())
        .spawn(|| {
            load_data(from_path_gen, to_analysis);
        })?;

    let anal_jh = thread::Builder::new()
        .name("analysis-findfire".to_owned())
        .spawn(|| {
            analyze_for_clusters(from_load_thread, to_database_thread);
        })?;

    let db_jh = thread::Builder::new()
        .name("writer-findfire".to_owned())
        .spawn(|| write_to_db(from_analysis_thread))?;

    match path_gen_jh.join() {
        Ok(()) => log::debug!("Path generation thread joined successfully."),
        Err(err) => log::error!("Error joining path generation thread: {:?}", err),
    }

    match load_jh.join() {
        Ok(()) => log::debug!("Load thread joined successfully."),
        Err(err) => log::error!("Error joining load thread: {:?}", err),
    }

    match anal_jh.join() {
        Ok(()) => log::debug!("Analysis thread joined successfully."),
        Err(err) => log::error!("Error joining analysis thread: {:?}", err),
    }

    match db_jh.join() {
        Ok(Some(biggest_fire)) => {
            log::debug!("Database writer thread joined successfully.");

            let Cluster {
                scan_start_time,
                satellite,
                sector,
                power,
                centroid,
                count,
                ..
            } = biggest_fire;
            let (lat, lon) = (centroid.x(), centroid.y());

            log::info!("");
            log::info!("Biggest fire added to database:");
            log::info!(
                "     satellite - {:>19}",
                Into::<&'static str>::into(satellite)
            );
            log::info!(
                "        sector - {:>19}",
                Into::<&'static str>::into(sector)
            );
            log::info!("    scan start - {:>19}", scan_start_time);
            log::info!("      latitude - {:>19.6}", lat);
            log::info!("     longitude - {:>19.6}", lon);
            log::info!("    power (MW) - {:>19.1}", power);
            log::info!("         count - {:>19}", count);
            log::info!("");
        }
        Ok(None) => {
            log::warn!("");
            log::warn!("No new clusters added to the database!");
            log::warn!("");
        }
        Err(err) => log::error!("Error joining Database writer thread: {:?}", err),
    };

    Ok(())
}

fn generate_paths(to_load_thread: Sender<walkdir::DirEntry>) {
    let cluster_db = match ClustersDatabase::connect(DATABASE_FILE) {
        Ok(handle) => handle,
        Err(err) => {
            log::error!("Error connecting to {} : {}", DATABASE_FILE, err);
            return;
        }
    };

    let mut most_recent: HashMap<String, NaiveDateTime> = HashMap::new();
    let beginning_of_time: NaiveDateTime = chrono::NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0);

    for sat in Satellite::iter() {
        for sect in Sector::iter() {
            let key = format!("{}_{}", sat, sect);
            let latest_entry = match cluster_db.find_latest_cluster(sat, sect) {
                Ok(vt) => vt,
                Err(err) => {
                    log::debug!("No latest entry for {}: {}", key, err);
                    continue;
                }
            };

            log::debug!("latest entry for {} is {}", key, latest_entry);
            most_recent.insert(key, latest_entry);
        }
    }

    // We don't need it anymore once we've loaded the data for the hashmap.
    drop(cluster_db);
    let most_recent = most_recent;

    for (entry, fname) in walkdir::WalkDir::new(DATA_DIR)
        .into_iter()
        .filter_map(|res| res.ok())
        // Ignore directories, WalkDir will take care of recursing into them.
        .filter(|entry| {
            let path = entry.path();

            if path.is_dir() && entry.depth() <= 3 {
                log::info!("Processing directory {:?}", entry.path());
            }

            path.is_file()
        })
        // Get the file name
        .map(|entry| {
            let fname: String = entry.file_name().to_string_lossy().to_string();
            (entry, fname)
        })
        // Only consider NetCDF files.
        .filter(|(_entry, fname)| fname.ends_with(".nc"))
        // Skip meso-sector files (for now)
        .filter(|(_entry, fname)| !fname.contains("FDCM"))
        // Filter out stuff older than the most recent in the database.
        .filter(|(_entry, fname)| {
            let mut most_recent_in_db = beginning_of_time;
            for sat in Satellite::iter() {
                for sect in Sector::iter() {
                    let sat: &'static str = sat.into();
                    let sect: &'static str = sect.into();

                    if fname.contains(sat) && fname.contains(sect) {
                        let key = format!("{}_{}", sat, sect);
                        match most_recent.get(&key) {
                            Some(mr) => most_recent_in_db = *mr,
                            None => break,
                        }
                    }
                }
            }

            let scan_start: NaiveDateTime = match FireSatImage::find_start_time(&fname) {
                Ok(st) => st,
                Err(err) => {
                    log::error!("Error parsing file name: {}\n   {}", fname, err);
                    return false;
                }
            };

            scan_start > most_recent_in_db
        })
    {
        log::trace!("Processing {}", fname);
        match to_load_thread.send(entry) {
            Ok(()) => {}
            Err(err) => {
                log::error!("Error sending to the load thread: {}", err);
                return;
            }
        }
    }
}

fn load_data(from_path_gen: Receiver<walkdir::DirEntry>, to_analysis: Sender<FireSatImage>) {
    for entry in from_path_gen {
        let fsat_data = match FireSatImage::open(entry.path()) {
            Ok(data) => data,
            Err(err) => {
                log::error!("Error loading {:?} : {}", entry.path(), err);
                continue;
            }
        };

        match to_analysis.send(fsat_data) {
            Ok(()) => {}
            Err(err) => {
                log::error!("Error sending to the load thread: {}", err);
                return;
            }
        }
    }
}

fn analyze_for_clusters(
    from_load_thread: Receiver<FireSatImage>,
    to_database_thread: Sender<Vec<Cluster>>,
) {
    for fire_sat_image in from_load_thread {
        let clusters = match Cluster::from_fire_sat_image(&fire_sat_image) {
            Ok(clusters) => clusters,
            Err(err) => {
                log::error!("Error analyzing clusters: {}", err);
                continue;
            }
        };

        match to_database_thread.send(clusters) {
            Ok(()) => {}
            Err(err) => {
                log::error!("Error sending to the load thread: {}", err);
                return;
            }
        }
    }
}

fn write_to_db(from_analysis_thread: Receiver<Vec<Cluster>>) -> Option<Cluster> {
    let cluster_db = match ClustersDatabase::connect(DATABASE_FILE) {
        Ok(handle) => handle,
        Err(err) => {
            log::error!(
                "Unable to connect to database file {} : {}",
                DATABASE_FILE,
                err
            );
            return None;
        }
    };

    let mut add_transaction = match cluster_db.add_cluster_handle() {
        Ok(handle) => handle,
        Err(err) => {
            log::error!("Unable to create add transaction: {}", err);
            return None;
        }
    };

    let mut biggest_fire: Option<Cluster> = None;

    for cluster_list in from_analysis_thread {
        for cluster in cluster_list {
            if let Some(ref big_fire) = biggest_fire {
                if big_fire.power < cluster.power {
                    biggest_fire = Some(cluster.clone());
                }
            } else {
                biggest_fire = Some(cluster.clone());
            }

            match add_transaction.add_cluster(
                cluster.satellite,
                cluster.sector,
                cluster.scan_start_time,
                cluster.centroid,
                cluster.power,
                cluster.perimeter,
                cluster.count,
            ) {
                Ok(()) => {}
                Err(err) => {
                    log::error!("Unable to add a cluster: {}", err);
                    return None;
                }
            }
        }
    }

    biggest_fire
}
