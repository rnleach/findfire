use std::error::Error;

use chrono::{Duration, NaiveDateTime};
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

    let mut num_new_non_child_fires = 0;
    for record in records {
        let next_time_stamp = record.scan_time;

        if next_time_stamp != curr_time_stamp {
            log::trace!(
                "Old time: {}  New time: {}",
                curr_time_stamp,
                next_time_stamp
            );
            assert!(next_time_stamp > curr_time_stamp);

            let new_child_fires = finish_this_time_step(&mut fires, &mut cluster_code_associations);
            log::debug!(
                "Number of fires: {:6} with {:3} new fires and {:3} new child fires at {}",
                fires.len(),
                num_new_non_child_fires,
                new_child_fires.len(),
                curr_time_stamp
            );

            fires.extend(new_child_fires);

            // TODO: Write out fire associations to a database

            curr_time_stamp = next_time_stamp;
            num_new_non_child_fires = 0;
        }

        // Try to assign it as a canidate member to a fire, but if that fails, create a new fire.
        if let Some(record) = assign_cluster_to_fire(&mut fires, record) {
            let id = next_fire_state.get_next_fire_id()?;
            let ClusterRecord {
                lat,
                lon,
                scan_time,
                radius,
                ..
            } = record;

            let fd = FireData {
                id,
                lat,
                lon,
                last_observed: scan_time,
                max_radius: radius,
                candidates: vec![],
                next_child_num: 0,
            };
            fires.push(fd);
            num_new_non_child_fires += 1;
        }
    }

    let new_fires = finish_this_time_step(&mut fires, &mut cluster_code_associations);
    fires.extend(new_fires);
    // TODO: Write out fire associations to a database

    fires.sort_by(|a, b| a.id.cmp(&b.id));

    let mut most_descendendant = fires[0].clone();

    for fire in fires {
        log::info!(
            "{:10.6} {:11.6} {:6.3} {:<}",
            fire.lat,
            fire.lon,
            fire.max_radius,
            fire.id.0
        );

        if fire.id.0.len() > most_descendendant.id.0.len() {
            most_descendendant = fire;
        }
    }
    log::info!("");
    log::info!("Tallest Family Tree");
    log::info!(
        "{:10.6} {:11.6} {:6.3} {} {:<}",
        most_descendendant.lat,
        most_descendendant.lon,
        most_descendendant.max_radius,
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
) -> Option<ClusterRecord> {
    // Close enough in kilometers
    const CLOSE_ENOUGH: f64 = 5.0;

    // If the last time the fire was observed was longer than this, lets assume this is a new
    // a new fire.
    let too_long_ago = cluster.scan_time - Duration::days(30);

    let mut closest: Option<&mut FireData> = None;
    let mut closest_distance: Option<f64> = None;
    for fire in fires
        .iter_mut()
        .rev()
        .filter(|f| f.last_observed > too_long_ago)
    {
        let distance =
            (satfire::great_circle_distance(fire.lat, fire.lon, cluster.lat, cluster.lon)
                - fire.max_radius)
                .max(0.0);

        if let Some(closest_so_far) = closest_distance {
            if distance < closest_so_far {
                closest_distance = Some(distance);
                closest = Some(fire)
            }
        } else {
            closest_distance = Some(distance);
            closest = Some(fire);
        }
    }

    if let (Some(closest_distance), Some(closest)) = (closest_distance, closest) {
        if closest_distance < CLOSE_ENOUGH {
            closest.candidates.push((cluster, closest_distance));
            None
        } else {
            Some(cluster)
        }
    } else {
        Some(cluster)
    }
}

#[derive(Debug, Clone)]
struct FireData {
    /// Unique string to id this fire. These will use a prefix code to show which fires are related.
    id: FireCode,

    /// Original cluster center
    lat: f64,
    lon: f64,

    /// The last time stamp that this fire was observed.
    last_observed: NaiveDateTime,

    /// Sum of the radius of the last cluster added and the distance from this center to that
    /// cluster's center.
    max_radius: f64,

    /// Potential candidates for a child fire
    candidates: Vec<(ClusterRecord, f64)>,

    /// Where to start numbering future children
    next_child_num: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FireCode(String);

impl FireCode {
    pub fn make_child_fire(&self, child_num: u32) -> FireCode {
        assert!(child_num < 1_000);

        FireCode(format!("{}{:03}", self.0, child_num))
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
    associations: &mut Vec<(ClusterRecord, FireCode)>,
) -> Vec<FireData> {
    let mut new_fires = vec![];

    for fire in fires.iter_mut().filter(|f| !f.candidates.is_empty()) {
        if fire.candidates.len() == 1 {
            // If there is only 1 child fire, update the radius & last observed date
            for (candidate, distance) in fire.candidates.drain(..) {
                fire.last_observed = candidate.scan_time;
                fire.max_radius = fire.max_radius.max(distance + candidate.radius);
                associations.push((candidate, fire.id.clone()));
            }
        } else {
            // If there are several candidates, create a new fire for each with an updated code
            for (candidate, _) in fire.candidates.drain(..) {
                let id = fire.id.make_child_fire(fire.next_child_num);
                fire.next_child_num += 1;
                associations.push((candidate, id.clone()));

                let ClusterRecord {
                    lat,
                    lon,
                    scan_time,
                    radius,
                    ..
                } = candidate;

                let new_fire = FireData {
                    id,
                    lat,
                    lon,
                    max_radius: radius,
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
