use std::{collections::HashMap, error::Error};

use findfire::{Cluster, ClusterDatabase, ClusterList, FireSatImage};

use chrono::NaiveDateTime;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";
const DATA_DIR: &'static str = "/home/ryan/wxdata/GOES/";

fn main() -> Result<(), Box<dyn Error>> {
    let cluster_db = ClusterDatabase::connect(DATABASE_FILE)?;
    let mut add_transaction = cluster_db.prepare()?;

    let mut biggest_fire = Cluster::default();

    let mut most_recent: HashMap<String, NaiveDateTime> = HashMap::new();

    let beginning_of_time: NaiveDateTime = chrono::NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0);

    for sat in ["G16", "G17"] {
        for sect in ["FDCC", "FDCM", "FDCF"] {
            let key = format!("{}_{}", sat, sect);
            let latest_entry = match cluster_db.find_latest(sat, sect) {
                Ok(vt) => vt,
                Err(_err) => continue,
            };

            most_recent.insert(key, latest_entry);
        }
    }

    for (entry, fname) in walkdir::WalkDir::new(DATA_DIR)
        .into_iter()
        .filter_map(|res| res.ok())
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
                    println!("Error parsing file name: {}\n   {}", fname, err);
                    return false;
                }
            };

            scan_start > most_recent_in_db
        })
    {
        println!("Processing: {}", fname);

        let clusters = ClusterList::from_file(entry.path())?;

        for cluster in &clusters.clusters {
            add_transaction.add_row(
                clusters.satellite,
                clusters.sector,
                clusters.start,
                cluster.lat,
                cluster.lon,
                cluster.power,
                cluster.radius,
                cluster.count,
            )?;

            if cluster.power > biggest_fire.power {
                biggest_fire = *cluster;
            }
        }
    }

    println!("{:?}", biggest_fire);

    Ok(())
}
