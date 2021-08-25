use std::error::Error;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";
const DATA_DIR: &'static str = "/home/ryan/wxdata/GOES/";

fn main() -> Result<(), Box<dyn Error>> {
    let cluster_db = crate::database::ClusterDatabase::connect(DATABASE_FILE)?;
    let mut add_transaction = cluster_db.prepare()?;

    let mut biggest_fire = crate::cluster::Cluster::default();

    for entry in walkdir::WalkDir::new(DATA_DIR)
        .into_iter()
        .filter_map(|res| res.ok())
        .filter(|entry| {
            let fname = entry.file_name().to_string_lossy();

            // Only consider NetCDF files.
            if !fname.ends_with(".nc") {
                return false;
            }

            // Skip full disk and meso-sector files (for now)
            if fname.contains("FDCF") || fname.contains("FDCM") {
                return false;
            }

            true
        })
    {
        let fname = entry.file_name();
        println!("Processing: {}", fname.to_string_lossy());

        let clusters = crate::cluster::ClusterList::from_file(entry.path())?;

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

mod cluster;
mod database;
mod error;
mod firepoint;
mod firesatimage;
mod geo;
