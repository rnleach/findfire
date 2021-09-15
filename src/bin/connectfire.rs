use std::{error::Error, path::Path, thread};

use chrono::{Duration, NaiveDateTime};
use crossbeam_channel::{bounded, Receiver, Sender};
use geo::{
    algorithm::{concave_hull::ConcaveHull, intersects::Intersects},
    line_string, polygon, MultiPolygon, Point, Polygon,
};
use itertools::Itertools;
use log::LevelFilter;
use satfire::{ClusterRecord, ClustersDatabase, FireCode, FiresDatabase};
use simple_logger::SimpleLogger;

const CLUSTERS_DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";
const FIRES_DATABASE_FILE: &'static str = "/home/ryan/wxdata/connectfire.sqlite";
const DAYS_FOR_FIRE_OUT: i64 = 21;
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
            read_database(CLUSTERS_DATABASE_FILE, "G17", to_fires_processing);
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
    Cluster(ClusterRecord),
    FinishTimeStep,
}

fn read_database<P: AsRef<Path>>(
    path_to_db: P,
    satellite: &'static str,
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

    for (curr_time_stamp, records) in records.group_by(|rec| rec.scan_time).into_iter() {
        match to_fires_processing.send(ClusterMessage::StartTimeStep(curr_time_stamp)) {
            Ok(()) => {}
            Err(err) => {
                log::error!("Error sending from read_database: {}", err);
                return;
            }
        }

        for record in records {
            match to_fires_processing.send(ClusterMessage::Cluster(record)) {
                Ok(()) => {}
                Err(err) => {
                    log::error!("Error sending from read_database: {}", err);
                    return;
                }
            }
        }

        match to_fires_processing.send(ClusterMessage::FinishTimeStep) {
            Ok(()) => {}
            Err(err) => {
                log::error!("Error sending from read_database: {}", err);
                return;
            }
        }
    }
}

#[derive(Debug, Clone)]
// TODO add satellite
struct FireData {
    /// Unique string to id this fire. These will use a prefix code to show which fires are related.
    id: FireCode,

    /// Original cluster center
    origin: Point<f64>,

    /// The last time stamp that this fire was observed.
    last_observed: NaiveDateTime,

    perimeter: Polygon<f64>,

    /// Potential candidates for a child fire
    candidates: Vec<ClusterRecord>,

    /// Where to start numbering future children
    next_child_num: u32,
}

enum DatabaseMessage {
    AddFire(FireData),
    AddAssociation(i64, FireCode),
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
                        // TODO check & log error
                        db_writer
                            .send(DatabaseMessage::AddFire(af.clone()))
                            .unwrap();
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

                    let ClusterRecord {
                        centroid,
                        scan_time,
                        perimeter,
                        ..
                    } = record;

                    let fd = FireData {
                        id: id.clone(),
                        origin: centroid,
                        last_observed: scan_time,
                        candidates: vec![],
                        next_child_num: 0,
                        perimeter,
                    };

                    // TODO check & log error
                    db_writer
                        .send(DatabaseMessage::AddFire(fd.clone()))
                        .unwrap();
                    // TODO check & log error
                    db_writer
                        .send(DatabaseMessage::AddAssociation(record.rowid, id))
                        .unwrap();

                    active_fires.push(fd);
                }
            }

            ClusterMessage::FinishTimeStep => {
                // Finish this time step
                finish_this_time_step(&mut active_fires, &db_writer);
            }
        }
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
                    next_child_num,
                    perimeter,
                    ..
                } = fire;

                match fires.add_fire(
                    id.clone_string(),
                    // TODO don't hard code "G17", get it from FireData
                    "G17",
                    last_observed,
                    origin,
                    perimeter.clone(),
                    next_child_num,
                ) {
                    Ok(()) => {}
                    Err(err) => {
                        log::error!("Error adding fire to database: {}", err);
                        return;
                    }
                }
            }

            DatabaseMessage::AddAssociation(cluster, fire_code) => {
                match cluster_code_associations.add_association(cluster, fire_code.clone_string()) {
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
    cluster: ClusterRecord,
    too_long_ago: NaiveDateTime,
) -> Option<ClusterRecord> {
    for fire in active_fires
        .iter_mut()
        .rev()
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
    let mut new_fires = vec![];

    let mut tmp_polygon: Polygon<f64> = polygon!();

    for fire in fires.iter_mut().filter(|f| !f.candidates.is_empty()) {
        if fire.candidates.len() == 1 {
            // If there is only 1 child fire, update the radius & last observed date
            for candidate in fire.candidates.drain(..) {
                fire.last_observed = candidate.scan_time;

                std::mem::swap(&mut tmp_polygon, &mut fire.perimeter);
                tmp_polygon = merge_polygons(tmp_polygon, candidate.perimeter);
                std::mem::swap(&mut tmp_polygon, &mut fire.perimeter);

                // TODO check res & log error
                db_writer
                    .send(DatabaseMessage::AddAssociation(
                        candidate.rowid,
                        fire.id.clone(),
                    ))
                    .unwrap();
            }
        } else {
            // If there are several candidates, create a new fire for each with an updated code
            for candidate in fire.candidates.drain(..) {
                let id = fire.id.make_child_fire(fire.next_child_num);
                fire.next_child_num += 1;

                let ClusterRecord {
                    centroid,
                    scan_time,
                    perimeter,
                    ..
                } = candidate;

                let new_fire = FireData {
                    id,
                    origin: centroid,
                    perimeter,
                    last_observed: scan_time,
                    candidates: vec![],
                    next_child_num: 0,
                };

                // TODO check res & log error
                db_writer
                    .send(DatabaseMessage::AddFire(new_fire.clone()))
                    .unwrap();

                // TODO check res & log error
                db_writer
                    .send(DatabaseMessage::AddAssociation(
                        candidate.rowid,
                        new_fire.id.clone(),
                    ))
                    .unwrap();

                new_fires.push(new_fire);
            }
        }
    }

    fires.extend(new_fires);
}

fn merge_polygons(left: Polygon<f64>, right: Polygon<f64>) -> Polygon<f64> {
    let mp = MultiPolygon::from(vec![left, right]);
    mp.concave_hull(2.0)
}
