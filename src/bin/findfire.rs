use std::{
    collections::HashMap,
    error::Error,
    thread::{self, JoinHandle},
};

use chrono::NaiveDateTime;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::LevelFilter;
use satfire::{Cluster, FireSatImage, FiresDatabase};
use simple_logger::SimpleLogger;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";
const DATA_DIR: &'static str = "/media/ryan/SAT/GOESX/";

const CHANNEL_SIZE: usize = 100;

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .with_module_level("findfire", LevelFilter::Debug)
        .init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let (to_load_thread, from_path_gen) = bounded(CHANNEL_SIZE);
    let (to_analysis, from_load_thread) = bounded(CHANNEL_SIZE);
    let (to_database_thread, from_analysis_thread) = bounded(CHANNEL_SIZE);

    let path_gen = start_path_generation_thread(to_load_thread)?;
    let load_thread = start_load_thread(from_path_gen, to_analysis)?;
    let anal_thread = start_analysis_thread(from_load_thread, to_database_thread)?;
    let db_thread = start_database_thread(from_analysis_thread)?;

    path_gen.join().unwrap();
    load_thread.join().unwrap();
    anal_thread.join().unwrap();
    let biggest_fire = db_thread.join().unwrap();

    if let Some(Cluster {
        scan_start_time,
        satellite,
        sector,
        power,
        centroid,
        count,
        ..
    }) = biggest_fire
    {
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
    } else {
        log::warn!("");
        log::warn!("No new clusters added to the database!");
        log::warn!("");
    }

    Ok(())
}

fn start_path_generation_thread(
    to_load_thread: Sender<walkdir::DirEntry>,
) -> Result<JoinHandle<()>, Box<dyn Error>> {
    let cluster_db = FiresDatabase::connect(DATABASE_FILE)?;

    let mut most_recent: HashMap<String, NaiveDateTime> = HashMap::new();

    let beginning_of_time: NaiveDateTime = chrono::NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0);

    for sat in ["G16", "G17"] {
        for sect in ["FDCC", "FDCM", "FDCF"] {
            let key = format!("{}_{}", sat, sect);
            let latest_entry = match cluster_db.find_latest_cluster(sat, sect) {
                Ok(vt) => vt,
                Err(err) => {
                    log::debug!("Error finding latest entry for {}: {}", key, err);
                    continue;
                }
            };

            log::debug!("latest entry for {} is {}", key, latest_entry);
            most_recent.insert(key, latest_entry);
        }
    }

    let jh = thread::Builder::new()
        .name("findfire-path_gen".to_owned())
        .spawn(move || {
            for (entry, fname) in walkdir::WalkDir::new(DATA_DIR)
                .into_iter()
                .filter_map(|res| res.ok())
                // Ignore directories, WalkDir will take care of recursing into them.
                .filter(|entry| entry.path().is_file())
                // Get the file name
                .map(|entry| {
                    let fname: String = entry.file_name().to_string_lossy().to_string();
                    (entry, fname)
                })
                // Only consider NetCDF files.
                .filter(|(_entry, fname)| fname.ends_with(".nc"))
                // Skip full disk and meso-sector files (for now)
                .filter(|(_entry, fname)| !(fname.contains("FDCF") || fname.contains("FDCM")))
                // Filter out stuff older than the most recent in the database.
                .filter(|(_entry, fname)| {
                    let mut most_recent_in_db = beginning_of_time;
                    for sat in ["G16", "G17"] {
                        for sect in ["FDCC", "FDCM", "FDCF"] {
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
                log::debug!("Processing {}", fname);
                to_load_thread.send(entry).unwrap();
            }
        })?;

    Ok(jh)
}

fn start_load_thread(
    from_path_gen: Receiver<walkdir::DirEntry>,
    to_analysis: Sender<FireSatImage>,
) -> Result<JoinHandle<()>, Box<dyn Error>> {
    let jh = thread::Builder::new()
        .name("findfire-load".to_owned())
        .spawn(move || {
            for entry in from_path_gen {
                let fsat_data = FireSatImage::open(entry.path()).unwrap();

                to_analysis.send(fsat_data).unwrap();
            }
        })?;

    Ok(jh)
}

fn start_analysis_thread(
    from_load_thread: Receiver<FireSatImage>,
    to_database_thread: Sender<Vec<Cluster>>,
) -> Result<JoinHandle<()>, Box<dyn Error>> {
    let jh = thread::Builder::new()
        .name("findfire-analysis".to_owned())
        .spawn(move || {
            for fire_sat_image in from_load_thread {
                let clusters = Cluster::from_fire_sat_image(&fire_sat_image).unwrap();
                to_database_thread.send(clusters).unwrap();
            }
        })?;

    Ok(jh)
}

fn start_database_thread(
    from_analysis_thread: Receiver<Vec<Cluster>>,
) -> Result<JoinHandle<Option<Cluster>>, Box<dyn Error>> {
    let jh = thread::Builder::new()
        .name("findfire-database".to_owned())
        .spawn(move || {
            let cluster_db = FiresDatabase::connect(DATABASE_FILE).unwrap();
            let mut add_transaction = cluster_db.add_cluster_handle().unwrap();

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

                    add_transaction
                        .add_cluster(
                            cluster.satellite,
                            cluster.sector,
                            cluster.scan_start_time,
                            cluster.centroid,
                            cluster.power,
                            cluster.perimeter,
                            cluster.count,
                        )
                        .unwrap();
                }
            }

            biggest_fire
        })?;

    Ok(jh)
}
