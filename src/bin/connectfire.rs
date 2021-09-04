use std::error::Error;

use chrono::NaiveDateTime;
use satfire::{ClusterDatabase, ClusterRecord, ConnectFireError};
use simple_logger::SimpleLogger;

const DATABASE_FILE: &'static str = "/home/ryan/wxdata/findfire.sqlite";

fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().init()?;

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

    for record in records {
        let next_time_stamp = record.scan_time;

        if next_time_stamp != curr_time_stamp {
            finish_this_time_step(&mut fires, &mut cluster_code_associations);
            // TODO: Write out fire associations to a database

            curr_time_stamp = next_time_stamp;
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
            };
            fires.push(fd);
        }
    }

    finish_this_time_step(&mut fires, &mut cluster_code_associations);
    // TODO: Write out fire associations to a database

    Ok(())
}

/// Return the ClusterRecord if it couldn't be assigned somewhere else
fn assign_cluster_to_fire(
    fires: &mut Vec<FireData>,
    cluster: ClusterRecord,
) -> Option<ClusterRecord> {
    unimplemented!()
}

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
    candidates: Vec<ClusterRecord>,
}

#[derive(Debug, Clone)]
struct FireCode(String);

impl FireCode {
    pub fn make_child_fire(&self, child_num: usize) -> FireCode {
        unimplemented!()
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
) {
    // TODO update data & make new fires if necessary

    for fire in fires.iter_mut() {
        // TODO: If there is only 1 child fire, update the radius & last observed date
        unimplemented!();

        // TODO: If there are several candidates, create a new fire for each with an updated code
        unimplemented!();

        // TODO: In all cases, update the associations
        unimplemented!();
    }
}
