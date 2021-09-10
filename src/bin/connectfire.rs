use std::error::Error;

use chrono::{Duration, NaiveDateTime};
use geo::{
    algorithm::{concave_hull::ConcaveHull, intersects::Intersects},
    line_string, polygon, MultiPolygon, Point, Polygon,
};
use itertools::Itertools;
use log::LevelFilter;
use satfire::{
    AddAssociationsTransaction, AddFireTransaction, ClusterRecord, FireCode, FiresDatabase,
};
use simple_logger::SimpleLogger;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";
const DAYS_FOR_FIRE_OUT: i64 = 21;

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().with_level(LevelFilter::Debug).init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let fires_db = FiresDatabase::connect(DATABASE_FILE)?;
    let mut records = fires_db.cluster_query_handle()?;
    let mut cluster_code_associations = fires_db.add_association_handle()?;
    let mut fires = fires_db.add_fire_handle()?;
    let mut next_fire_state = fires_db.next_new_fire_id_state()?;

    let mut last_purge: NaiveDateTime = chrono::NaiveDate::from_ymd(1900, 1, 1).and_hms(0, 0, 0);

    let mut active_fires = vec![];

    for (curr_time_stamp, records) in records
        .records_for("G17")?
        .group_by(|record| record.scan_time)
        .into_iter()
    {
        let too_long_ago = curr_time_stamp - Duration::days(DAYS_FOR_FIRE_OUT);

        if curr_time_stamp - last_purge > Duration::days(1) {
            for af in active_fires
                .iter()
                .filter(|af: &&FireData| af.last_observed <= too_long_ago)
            {
                fires.add_fire(
                    af.id.clone_string(),
                    "G17",
                    af.last_observed,
                    af.origin,
                    af.perimeter.clone(),
                )?;
            }

            active_fires.retain(|f: &FireData| f.last_observed > too_long_ago);

            last_purge = curr_time_stamp;
        }

        let mut num_fires = 0;
        let mut num_new_fires = 0;

        for record in records {
            num_fires += 1;

            // Try to assign it as a canidate member to a fire, but if that fails, create a new fire.
            if let Some(record) = assign_cluster_to_fire(&mut active_fires, record, too_long_ago) {
                let id = next_fire_state
                    .get_next_fire_id()
                    .expect("Ran out of fire ID #'s!");
                let ClusterRecord {
                    centroid,
                    scan_time,
                    perimeter,
                    ..
                } = record;

                cluster_code_associations
                    .add_association(record.rowid, id.clone_string())
                    .unwrap();

                let fd = FireData {
                    id,
                    origin: centroid,
                    last_observed: scan_time,
                    candidates: vec![],
                    next_child_num: 0,
                    perimeter,
                };
                fires.add_fire(
                    fd.id.clone_string(),
                    "G17",
                    fd.last_observed,
                    fd.origin,
                    fd.perimeter.clone(),
                )?;
                active_fires.push(fd);
                num_new_fires += 1;
            }
        }

        let num_old_fires = num_fires - num_new_fires;
        log::debug!(
            "[{}] Fires this time: {:4}  Old: {:4} ({:3.0}%) New: {:4} ({:3.0}%) Total fires: {:6}",
            curr_time_stamp,
            num_fires,
            num_old_fires,
            num_old_fires as f64 / num_fires as f64 * 100.0,
            num_new_fires,
            num_new_fires as f64 / num_fires as f64 * 100.0,
            active_fires.len(),
        );

        finish_this_time_step(
            &mut active_fires,
            &mut cluster_code_associations,
            &mut fires,
            "G17",
        )?;
    }

    for af in &active_fires {
        fires.add_fire(
            af.id.clone_string(),
            "G17",
            af.last_observed,
            af.origin,
            af.perimeter.clone(),
        )?;
    }

    if let Some(most_descendant) = active_fires
        .into_iter()
        .max_by_key(|item| item.id.num_generations())
    {
        log::info!("");
        log::info!("Tallest Family Tree");
        let (lat, lon) = (most_descendant.origin.x(), most_descendant.origin.y());
        log::info!(
            "{:10.6} {:11.6} {} {:2} {:<}",
            lat,
            lon,
            most_descendant.last_observed,
            most_descendant.id.num_generations(),
            most_descendant.id,
        );
        log::info!("");
    }

    Ok(())
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

#[derive(Debug, Clone)]
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

fn finish_this_time_step(
    fires: &mut Vec<FireData>,
    associations: &mut AddAssociationsTransaction,
    fires_db: &mut AddFireTransaction,
    satellite: &'static str,
) -> Result<(), Box<dyn Error>> {
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

                associations
                    .add_association(candidate.rowid, fire.id.clone_string())
                    .unwrap();
            }
        } else {
            // If there are several candidates, create a new fire for each with an updated code
            for candidate in fire.candidates.drain(..) {
                let id = fire.id.make_child_fire(fire.next_child_num);
                // TODO add the next child value to the
                fire.next_child_num += 1;
                associations
                    .add_association(candidate.rowid, fire.id.clone_string())
                    .unwrap();

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

                fires_db.add_fire(
                    new_fire.id.clone_string(),
                    satellite,
                    new_fire.last_observed,
                    new_fire.origin,
                    new_fire.perimeter.clone(),
                )?;
                new_fires.push(new_fire);
            }
        }
    }

    fires.extend(new_fires);

    Ok(())
}

fn merge_polygons(left: Polygon<f64>, right: Polygon<f64>) -> Polygon<f64> {
    let mp = MultiPolygon::from(vec![left, right]);
    mp.concave_hull(2.0)
}
