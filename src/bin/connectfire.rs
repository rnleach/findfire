use std::{error::Error, path::Path, thread};

use chrono::{Duration, NaiveDateTime};
use crossbeam_channel::{bounded, Receiver, Sender};
use geo::{
    algorithm::{concave_hull::ConcaveHull, intersects::Intersects},
    line_string, polygon, MultiPolygon, Point, Polygon,
};
use itertools::Itertools;
use log::LevelFilter;
use satfire::{Cluster, ClustersDatabase, FireCode, FiresDatabase, Satellite};
use simple_logger::SimpleLogger;

const CLUSTERS_DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";
const FIRES_DATABASE_FILE: &'static str = "/home/ryan/wxdata/connectfire.sqlite";
const DAYS_FOR_FIRE_OUT: i64 = 60;
const CHANNEL_SIZE: usize = 1_000;

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().with_level(LevelFilter::Debug).init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let (to_fires_processing, cluster_msgs) = bounded(CHANNEL_SIZE);
    let (db_writer, db_messages) = bounded(CHANNEL_SIZE);

    let reader_jh = thread::Builder::new()
        .name("reader-connectfire".to_owned())
        .spawn(move || {
            read_database(CLUSTERS_DATABASE_FILE, Satellite::G17, to_fires_processing);
        })?;

    let processing_jh = thread::Builder::new()
        .name("processing-connectfire".to_owned())
        .spawn(move || {
            process_fires(FIRES_DATABASE_FILE, cluster_msgs, db_writer);
        })?;

    let writer_jh = thread::Builder::new()
        .name("writer-connectfire".to_owned())
        .spawn(move || {
            write_to_database(FIRES_DATABASE_FILE, db_messages);
        })?;

    match reader_jh.join() {
        Ok(_) => log::debug!("Reader thread joined successfully"),
        Err(err) => log::error!("Reader thread failed: {:?}", err),
    }

    match processing_jh.join() {
        Ok(_) => log::debug!("Processing thread joined successfully"),
        Err(err) => log::error!("Processing thread failed: {:?}", err),
    }

    match writer_jh.join() {
        Ok(_) => log::debug!("Writer thread joined successfully"),
        Err(err) => log::error!("Writer thread failed: {:?}", err),
    }

    Ok(())
}

enum ClusterMessage {
    StartTimeStep(NaiveDateTime),
    Cluster(Cluster),
    FinishTimeStep,
}

enum DatabaseMessage {
    AddFire(FireData),
    AddAssociation(FireCode, NaiveDateTime, f64, Polygon<f64>),
}

#[derive(Debug, Clone)]
struct FireData {
    /// Unique string to id this fire. These will use a prefix code to show which fires are related.
    id: FireCode,

    /// The satellite that this fire was detected by.
    satellite: Satellite,

    /// Original cluster center
    origin: Point<f64>,

    /// The last time stamp that this fire was observed.
    last_observed: NaiveDateTime,

    perimeter: Polygon<f64>,

    /// Potential candidates for a child fire
    candidates: Vec<Cluster>,
}

macro_rules! send_or_return {
    ($channel:ident, $item:expr, $msg:expr) => {
        match $channel.send($item) {
            Ok(()) => {}
            Err(err) => {
                log::error!($msg, err);
                return;
            }
        }
    };
}

fn read_database<P: AsRef<Path>>(
    path_to_db: P,
    satellite: Satellite,
    to_fires_processing: Sender<ClusterMessage>,
) {
    let clusters_db = match ClustersDatabase::connect(path_to_db) {
        Ok(db) => db,
        Err(err) => {
            log::error!("Error connecting to read database: {}", err);
            return;
        }
    };

    let mut records = match clusters_db.cluster_query_handle() {
        Ok(records) => records,
        Err(err) => {
            log::error!("Error querying data base to read cluster records: {}", err);
            return;
        }
    };

    let records = match records.records_for(satellite) {
        Ok(records) => records,
        Err(err) => {
            log::error!("Error querying data base to read cluster records: {}", err);
            return;
        }
    };

    for (curr_time_stamp, records) in records.group_by(|rec| rec.scan_start_time).into_iter() {
        send_or_return!(
            to_fires_processing,
            ClusterMessage::StartTimeStep(curr_time_stamp),
            "Error sending from read_database: {}"
        );

        for record in records {
            send_or_return!(
                to_fires_processing,
                ClusterMessage::Cluster(record),
                "Error sending from read_database: {}"
            );
        }

        send_or_return!(
            to_fires_processing,
            ClusterMessage::FinishTimeStep,
            "Error sending from read_database: {}"
        );
    }
}

fn process_fires<P: AsRef<Path>>(
    path_to_db: P,
    clusters_msgs: Receiver<ClusterMessage>,
    db_writer: Sender<DatabaseMessage>,
) {
    let fires_db = match FiresDatabase::connect(path_to_db) {
        Ok(db) => db,
        Err(err) => {
            log::error!("Error connecting to database: {}", err);
            return;
        }
    };

    let mut next_fire_state = match fires_db.next_new_fire_id_state() {
        Ok(db) => db,
        Err(err) => {
            log::error!("Error connecting to database: {}", err);
            return;
        }
    };

    let mut active_fires = vec![];
    let mut last_purge: NaiveDateTime = chrono::NaiveDate::from_ymd(1900, 1, 1).and_hms(0, 0, 0);
    let mut too_long_ago = last_purge;

    for msg in clusters_msgs {
        match msg {
            ClusterMessage::StartTimeStep(curr_time_stamp) => {
                // Update flags for this time step
                too_long_ago = curr_time_stamp - Duration::days(DAYS_FOR_FIRE_OUT);

                if curr_time_stamp - last_purge > Duration::days(1) {
                    for af in active_fires
                        .iter()
                        .filter(|af: &&FireData| af.last_observed <= too_long_ago)
                    {
                        send_or_return!(
                            db_writer,
                            DatabaseMessage::AddFire(af.clone()),
                            "Error sending to db_writer: {}"
                        );
                    }

                    active_fires.retain(|f: &FireData| f.last_observed > too_long_ago);

                    last_purge = curr_time_stamp;
                }
            }

            ClusterMessage::Cluster(record) => {
                // Try to assign it as a canidate member to a fire, but if that fails, create a new fire.
                if let Some(record) =
                    assign_cluster_to_fire(&mut active_fires, record, too_long_ago)
                {
                    let id = next_fire_state
                        .get_next_fire_id()
                        .expect("Ran out of fire ID #'s!");

                    let Cluster {
                        centroid,
                        scan_start_time,
                        perimeter,
                        power,
                        satellite,
                        ..
                    } = record;

                    let fd = FireData {
                        id: id.clone(),
                        origin: centroid,
                        last_observed: scan_start_time,
                        candidates: vec![],
                        perimeter: perimeter.clone(),
                        satellite,
                    };

                    send_or_return!(
                        db_writer,
                        DatabaseMessage::AddAssociation(id, scan_start_time, power, perimeter),
                        "Error sending to db_writer: {}"
                    );

                    active_fires.push(fd);
                }
            }

            ClusterMessage::FinishTimeStep => {
                // Finish this time step
                finish_this_time_step(&mut active_fires, &db_writer);
            }
        }
    }

    for fire in active_fires.drain(..) {
        send_or_return!(
            db_writer,
            DatabaseMessage::AddFire(fire),
            "Error sending to db_writer: {}"
        );
    }
}

fn write_to_database<P: AsRef<Path>>(path_to_db: P, messages: Receiver<DatabaseMessage>) {
    let fires_db = match FiresDatabase::connect(path_to_db) {
        Ok(db) => db,
        Err(err) => {
            log::error!("Error connecting to database: {}", err);
            return;
        }
    };

    let mut fires = match fires_db.add_fire_handle() {
        Ok(handle) => handle,
        Err(err) => {
            log::error!("Error getting add fire handle: {}", err);
            return;
        }
    };

    let mut cluster_code_associations = match fires_db.add_association_handle() {
        Ok(handle) => handle,
        Err(err) => {
            log::error!("Error getting cluster-code-associations handle: {}", err);
            return;
        }
    };

    for msg in messages {
        match msg {
            DatabaseMessage::AddFire(fire) => {
                let FireData {
                    id,
                    origin,
                    last_observed,
                    perimeter,
                    satellite,
                    ..
                } = fire;

                match fires.add_fire(id, satellite, last_observed, origin, perimeter.clone()) {
                    Ok(()) => {}
                    Err(err) => {
                        log::error!("Error adding fire to database: {}", err);
                        return;
                    }
                }
            }

            DatabaseMessage::AddAssociation(fire_code, scan_time, power, perimeter) => {
                match cluster_code_associations
                    .add_association(fire_code, scan_time, power, perimeter)
                {
                    Ok(()) => {}
                    Err(err) => {
                        log::error!("Error adding fire association to database: {}", err);
                        return;
                    }
                }
            }
        }
    }
}

/// Return the ClusterRecord if it couldn't be assigned somewhere else
fn assign_cluster_to_fire(
    active_fires: &mut Vec<FireData>,
    cluster: Cluster,
    too_long_ago: NaiveDateTime,
) -> Option<Cluster> {
    for fire in active_fires
        .iter_mut()
        .rev()
        // No mixing and matching between satllites.
        .filter(|f| f.satellite == cluster.satellite)
        .take_while(|f| f.last_observed > too_long_ago)
    {
        if fire.perimeter.intersects(&cluster.perimeter) {
            fire.candidates.push(cluster);
            return None;
        }
    }

    Some(cluster)
}

fn finish_this_time_step(fires: &mut Vec<FireData>, db_writer: &Sender<DatabaseMessage>) {
    let mut tmp_polygon: Polygon<f64> = polygon!();

    for fire in fires.iter_mut().filter(|f| !f.candidates.is_empty()) {
        for candidate in fire.candidates.drain(..) {
            fire.last_observed = candidate.scan_start_time;

            std::mem::swap(&mut tmp_polygon, &mut fire.perimeter);
            tmp_polygon = merge_polygons(tmp_polygon, candidate.perimeter.clone());
            std::mem::swap(&mut tmp_polygon, &mut fire.perimeter);

            send_or_return!(
                db_writer,
                DatabaseMessage::AddAssociation(
                    fire.id.clone(),
                    candidate.scan_start_time,
                    candidate.power,
                    candidate.perimeter
                ),
                "Error sending to db_writer: {}"
            );
        }
    }
}

fn merge_polygons(left: Polygon<f64>, right: Polygon<f64>) -> Polygon<f64> {
    let mp = MultiPolygon::from(vec![left, right]);
    mp.concave_hull(2.0)
}
