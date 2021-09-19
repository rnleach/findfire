use std::{error::Error, path::Path, thread};

use chrono::{Duration, NaiveDateTime};
use crossbeam_channel::{bounded, Receiver, Sender};
use geo::{
    algorithm::{chamberlain_duquette_area::ChamberlainDuquetteArea, intersects::Intersects},
    line_string, polygon, MultiPolygon, Point,
};
use itertools::Itertools;
use kd_tree::{KdIndexTree, KdPoint};
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

enum DatabaseMessage {
    AddFire(FireData),
    AddAssociation(FireCode, NaiveDateTime, f64, MultiPolygon<f64>),
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

    perimeter: MultiPolygon<f64>,

    /// Potential candidates for a child fire
    candidates: Vec<Cluster>,
}

impl KdPoint for FireData {
    type Scalar = f64;
    type Dim = typenum::U2;

    fn at(&self, k: usize) -> Self::Scalar {
        match k {
            0 => self.origin.x(),
            1 => self.origin.y(),
            _ => unreachable!(),
        }
    }
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
    to_fires_processing: Sender<Cluster>,
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

    for record in records {
        send_or_return!(
            to_fires_processing,
            record,
            "Error sending from read_database: {}"
        );
    }
}

fn process_fires<P: AsRef<Path>>(
    path_to_db: P,
    clusters_msgs: Receiver<Cluster>,
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
    let mut new_fires = vec![];
    let mut assignments = vec![];
    let mut last_purge: NaiveDateTime = chrono::NaiveDate::from_ymd(1900, 1, 1).and_hms(0, 0, 0);

    for (curr_time_stamp, clusters) in clusters_msgs
        .into_iter()
        .group_by(|cluster| cluster.scan_start_time)
        .into_iter()
    {
        // Update flags for this time step
        let too_long_ago = curr_time_stamp - Duration::days(DAYS_FOR_FIRE_OUT);

        if curr_time_stamp - last_purge > Duration::days(1) {
            merge_fires(&mut active_fires, &db_writer);

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

        let kdtree = KdIndexTree::build_by_ordered_float(&active_fires);
        for record in clusters {
            if let Some(record) = assign_cluster_to_fire(&mut assignments, &kdtree, record) {
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

                new_fires.push(fd);
            }
        }

        active_fires.extend(new_fires.drain(..));

        // Finish this time step
        finish_this_time_step(&mut active_fires, &mut assignments, &db_writer);
    }

    merge_fires(&mut active_fires, &db_writer);
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
    assignments: &mut Vec<(usize, Cluster)>,
    active_fires: &KdIndexTree<FireData>,
    cluster: Cluster,
) -> Option<Cluster> {
    if let Some(fire_idx) = active_fires.nearest(&cluster) {
        let idx = *fire_idx.item;
        let fire = &active_fires.item(idx);

        if fire.perimeter.intersects(&cluster.perimeter) {
            assignments.push((idx, cluster));
            return None;
        }
    }

    Some(cluster)
}

fn finish_this_time_step(
    fires: &mut Vec<FireData>,
    assignments: &mut Vec<(usize, Cluster)>,
    db_writer: &Sender<DatabaseMessage>,
) {
    let mut tmp_polygon: MultiPolygon<f64> = MultiPolygon::from(vec![polygon!()]);

    for (i, cluster) in assignments.drain(..) {
        let fire = &mut fires[i];
        fire.last_observed = cluster.scan_start_time;

        std::mem::swap(&mut tmp_polygon, &mut fire.perimeter);
        tmp_polygon = merge_polygons(tmp_polygon, cluster.perimeter.clone());
        std::mem::swap(&mut tmp_polygon, &mut fire.perimeter);

        send_or_return!(
            db_writer,
            DatabaseMessage::AddAssociation(
                fire.id.clone(),
                cluster.scan_start_time,
                cluster.power,
                cluster.perimeter
            ),
            "Error sending to db_writer: {}"
        );
    }

    assert!(assignments.is_empty());
}

fn merge_fires(fires: &mut Vec<FireData>, db_writer: &Sender<DatabaseMessage>) {
    let mut mergers = vec![];
    let mut idxs_to_remove = vec![];

    let kdtree = KdIndexTree::build_by_ordered_float(&fires);
    for i in 0..fires.len() {

        // This polygon was already marked for merging into another one.
        if idxs_to_remove.contains(&i) {
            continue;
        }

        let curr_fire = &fires[i];

        let candidates: Vec<usize> = kdtree
            .nearests(curr_fire, 4)
            .into_iter()
            .map(|x| *x.item)
            // This would be the same one we're working on!
            .filter(|&j| j != i) 
            // This fire has already been marked to merge
            // See how this is handled below with a log message.
            //.filter(|j| idxs_to_remove.contains(j)) 
            .collect();

        for j in candidates {

            let candidate = &fires[j];

            if curr_fire.perimeter.intersects(&candidate.perimeter) {

                // This fire was already marked to merge with another fire! So we must have 
                // multiple overlaps. This should get picked up on the next round, but let's 
                // log it for now and see if it's a case we should maybe handle better in the
                // future. 
                if idxs_to_remove.contains(&j) {
                    log::warn!("Detected need for double merger, but not doing it!");
                    continue;
                }

                let curr_fire_area: f64 = curr_fire
                    .perimeter
                    .iter()
                    .map(|p| p.chamberlain_duquette_unsigned_area())
                    .sum();

                let candidate_area: f64 = candidate
                    .perimeter
                    .iter()
                    .map(|p| p.chamberlain_duquette_unsigned_area())
                    .sum();
                if curr_fire_area > candidate_area {
                    mergers.push((i, j));
                    idxs_to_remove.push(j);
                } else {
                    mergers.push((j, i));
                    idxs_to_remove.push(i);
                }
            }
        }
    }
    drop(kdtree);

    let mut tmp_polygon = MultiPolygon::from(vec![polygon!()]);
    for (i, j) in mergers {

        std::mem::swap(&mut tmp_polygon, &mut fires[i].perimeter);
        tmp_polygon = merge_polygons(tmp_polygon, fires[j].perimeter.clone());
        std::mem::swap(&mut tmp_polygon, &mut fires[i].perimeter);
    }

    idxs_to_remove.sort();

    // Remove fires that were smaller when merged.
    for idx in idxs_to_remove.into_iter().rev() {
        let fire = fires.swap_remove(idx);
        send_or_return!(
            db_writer,
            DatabaseMessage::AddFire(fire),
            "Error sending to db_writer: {}"
        );
    }
}

fn merge_polygons(left: MultiPolygon<f64>, right: MultiPolygon<f64>) -> MultiPolygon<f64> {
    let mut merged = left.0;
    merged.extend(right.0);

    merged.dedup();
    MultiPolygon::from(merged)
}
