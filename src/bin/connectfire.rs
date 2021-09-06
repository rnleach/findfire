use std::error::Error;

use chrono::{Duration, NaiveDateTime};
use geo::{
    algorithm::{concave_hull::ConcaveHull, intersects::Intersects},
    line_string, polygon, MultiPolygon, Point, Polygon,
};
use log::LevelFilter;
use satfire::{ClusterDatabase, ClusterRecord, ConnectFireError};
use simple_logger::SimpleLogger;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().with_level(LevelFilter::Debug).init()?;

    log::trace!("Trace messages enabled.");
    log::debug!("Debug messages enabled.");
    log::info!("Info messages enabled.");
    log::warn!("Warn messages enabled.");
    log::error!("Error messages enabled.");

    let cluster_db = ClusterDatabase::connect(DATABASE_FILE)?;
    let mut records = cluster_db.create_cluster_record_query()?;
    let records = records.cluster_records_for("G17")?;

    let mut curr_time_stamp = chrono::NaiveDate::from_ymd(1900, 1, 1).and_hms(0, 0, 0);
    let mut next_fire_state = FireDataNextNewFireState(1);
    let mut fires = vec![];
    let mut cluster_code_associations = vec![];

    // If the last time the fire was observed was longer than this, lets assume this is a new
    // a new fire.
    let mut too_long_ago = curr_time_stamp;
    let mut last_sort = curr_time_stamp;

    let mut num_new_non_child_fires = 0;
    let mut total_clusters_this_time_step = 0;
    for record in records {
        let next_time_stamp = record.scan_time;

        if next_time_stamp != curr_time_stamp {
            assert!(next_time_stamp > curr_time_stamp);

            let new_child_fires = finish_this_time_step(&mut fires, &mut cluster_code_associations);

            let num_old_fires =
                total_clusters_this_time_step - new_child_fires.len() - num_new_non_child_fires;

            log::debug!(
                "[{}] Fires this time: {:4}  Old: {:4} ({:3.0}%) New: {:4} ({:3.0}%) Children: {:4} ({:3.0}%)  Total fires: {:6}",
                curr_time_stamp,
                total_clusters_this_time_step,
                num_old_fires,
                num_old_fires as f64 / total_clusters_this_time_step as f64 * 100.0,
                num_new_non_child_fires,
                num_new_non_child_fires as f64 / total_clusters_this_time_step as f64 * 100.0,
                new_child_fires.len(),
                new_child_fires.len() as f64 / total_clusters_this_time_step as f64 * 100.0,
                fires.len(),
            );

            if curr_time_stamp - last_sort > Duration::days(1) {
                fires.sort_by_key(|k| k.last_observed );
                last_sort = curr_time_stamp;
            }

            fires.extend(new_child_fires);

            // TODO: Write out fire associations to a database

            curr_time_stamp = next_time_stamp;
            too_long_ago = curr_time_stamp - Duration::days(21);
            num_new_non_child_fires = 0;
            total_clusters_this_time_step = 0;
        }

        // Try to assign it as a canidate member to a fire, but if that fails, create a new fire.
        if let Some(record) = assign_cluster_to_fire(&mut fires, record, too_long_ago) {
            let id = next_fire_state.get_next_fire_id()?;
            let ClusterRecord {
                centroid,
                scan_time,
                perimeter,
                ..
            } = record;

            let fd = FireData {
                id,
                origin: centroid,
                last_observed: scan_time,
                candidates: vec![],
                next_child_num: 0,
                perimeter,
            };
            fires.push(fd);
            num_new_non_child_fires += 1;
        }

        total_clusters_this_time_step += 1;
    }

    let new_fires = finish_this_time_step(&mut fires, &mut cluster_code_associations);
    fires.extend(new_fires);
    // TODO: Write out fire associations to a database

    fires.sort_by(|a, b| a.id.cmp(&b.id));

    let mut most_descendendant = fires[0].clone();

    for fire in fires {
        let (lat, lon) = (fire.origin.x(), fire.origin.y());
        log::info!("{:10.6} {:11.6} {:<}", lat, lon, fire.id.0);

        if fire.id.0.len() > most_descendendant.id.0.len() {
            most_descendendant = fire;
        }
    }
    log::info!("");
    log::info!("Tallest Family Tree");
    let (lat, lon) = (most_descendendant.origin.x(), most_descendendant.origin.y());
    log::info!(
        "{:10.6} {:11.6} {} {:<}",
        lat,
        lon,
        most_descendendant.last_observed,
        most_descendendant.id.0
    );
    log::info!("");

    Ok(())
}

/// Return the ClusterRecord if it couldn't be assigned somewhere else
fn assign_cluster_to_fire(
    fires: &mut Vec<FireData>,
    cluster: ClusterRecord,
    too_long_ago: NaiveDateTime,
) -> Option<ClusterRecord> {
    for fire in fires
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FireCode(String);

impl FireCode {
    pub fn make_child_fire(&self, child_num: u32) -> FireCode {
        assert!(child_num < 100);

        FireCode(format!("{}{:02}", self.0, child_num))
    }
}

struct FireDataNextNewFireState(u32);

impl FireDataNextNewFireState {
    pub fn get_next_fire_id(&mut self) -> Result<FireCode, ConnectFireError> {
        let val_to_return = self.0;

        self.0 += 1;

        if val_to_return <= 999_999 {
            Ok(FireCode(format!("{:06}", val_to_return)))
        } else {
            Err(ConnectFireError {
                msg: "Too many fires for this program.",
            })
        }
    }
}

fn finish_this_time_step(
    fires: &mut Vec<FireData>,
    associations: &mut Vec<(i64, FireCode)>,
) -> Vec<FireData> {
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

                associations.push((candidate.rowid, fire.id.clone()));
            }
        } else {
            // If there are several candidates, create a new fire for each with an updated code
            for candidate in fire.candidates.drain(..) {
                let id = fire.id.make_child_fire(fire.next_child_num);
                fire.next_child_num += 1;
                associations.push((candidate.rowid, id.clone()));

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

                new_fires.push(new_fire);
            }
        }
    }

    new_fires
}

fn merge_polygons(left: Polygon<f64>, right: Polygon<f64>) -> Polygon<f64> {
    let mp = MultiPolygon::from(vec![left, right]);
    mp.concave_hull(2.0)
}
