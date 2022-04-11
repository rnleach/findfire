use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, Utc};
use clap::Parser;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{error, info, warn};
use satfire::{
    BoundingBox, ClusterDatabase, Coord, Fire, FireList, FireListUpdateResult, FireListView,
    FiresDatabase, SatFireResult, Satellite,
};
use simple_logger::SimpleLogger;
use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread::{self, JoinHandle},
};

use strum::IntoEnumIterator;

/*-------------------------------------------------------------------------------------------------
 *                                        Global State
 *-----------------------------------------------------------------------------------------------*/
static NEXT_WILDFIRE_ID: AtomicU64 = AtomicU64::new(0);
static SHUT_DOWN: AtomicBool = AtomicBool::new(false);

/*-------------------------------------------------------------------------------------------------
 *                                     Command Line Options
 *-----------------------------------------------------------------------------------------------*/

///
/// Create several time series of fires by temporally connecting clusters (from findfire).
///
/// Connect clusters from the output database of findfire to make time series' of fires. Each time
/// series is given an ID and stored in a database with a start date and an end date. In the
/// future, other statistics may be added to that database. Another table in the database will
/// record the relationship to clusters by associating a row number from the database with a fire ID
/// (created by this program) to the table with clusters.
///
#[derive(Debug, Parser)]
#[clap(bin_name = "connectfire")]
#[clap(author, version, about)]
struct ConnectFireOptions {
    /// The start date, do not try to connect clusters before this date.
    #[clap(short, long)]
    #[clap(parse(try_from_str=parse_datetime))]
    start: Option<DateTime<Utc>>,

    /// The end date, do not try to connect clusters after this date.
    #[clap(short, long)]
    #[clap(parse(try_from_str=parse_datetime))]
    end: Option<DateTime<Utc>>,

    /// Bounding Box where as bottom_lat,left_lon,top_lat,right_lon
    #[clap(parse(try_from_str=parse_bbox))]
    #[clap(default_value_t=BoundingBox{ll:Coord{lat: -90.0, lon: -180.0}, ur:Coord{lat: 90.0, lon: 180.0}})]
    bbox: BoundingBox,

    /// The path to the database file with the clusters.
    ///
    /// If this is not specified, then the program will check for it in the "CLUSTER_DB"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "CLUSTER_DB")]
    clusters_store_file: PathBuf,

    /// The path to the database file with the fires and associations.
    ///
    /// If this is not specified, then the program will check for it in the "FIRES_DB"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "FIRES_DB")]
    fires_store_file: PathBuf,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

/// Parse a bounding box argument.
fn parse_bbox(bbox_str: &str) -> SatFireResult<BoundingBox> {
    let corners: Vec<_> = bbox_str.split(',').collect();

    if corners.len() < 4 {
        return Err("Invalid number of coords".into());
    }

    let min_lat = corners[0].parse()?;
    let min_lon = corners[1].parse()?;
    let max_lat = corners[2].parse()?;
    let max_lon = corners[3].parse()?;

    if min_lat >= max_lat || min_lon >= max_lon {
        return Err(format!(
            concat!(
                "Minimum Lat/Lon must be less than Maximum Lat/Lon:",
                " min_lat={} max_lat={} min_lon={} max_lon={}"
            ),
            min_lat, max_lat, min_lon, max_lon
        )
        .into());
    }

    if min_lat < -90.0 || max_lat > 90.0 || min_lon < -180.0 || max_lon > 180.0 {
        return Err(format!(
            concat!(
                "Lat/Lon are out of range (-90.0 to 90.0 and -180.0 to 180.0):",
                " min_lat={} max_lat={} min_lon={} max_lon={}"
            ),
            min_lat, max_lat, min_lon, max_lon
        )
        .into());
    }

    let ll = Coord {
        lat: min_lat,
        lon: min_lon,
    };
    let ur = Coord {
        lat: max_lat,
        lon: max_lon,
    };

    Ok(BoundingBox { ll, ur })
}

/// Parse a command line datetime
fn parse_datetime(dt_str: &str) -> SatFireResult<DateTime<Utc>> {
    const TIME_FORMAT: &str = "%Y-%m-%d-%H:%M:%S";
    let t_str = format!("{}:00:00", dt_str);

    let naive = NaiveDateTime::parse_from_str(&t_str, TIME_FORMAT)?;
    Ok(DateTime::from_utc(naive, Utc))
}

impl Display for ConnectFireOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "\n")?; // yes, two blank lines.
        writeln!(
            f,
            "Cluster Database: {}",
            self.clusters_store_file.display()
        )?;
        writeln!(f, "  Fires Database: {}", self.fires_store_file.display())?;
        writeln!(f, "\n")?; // yes, two blank lines.

        Ok(())
    }
}

/// Get the command line arguments and check them.
///
/// If there is missing data, try to fill it in with environment variables.
fn parse_args() -> SatFireResult<ConnectFireOptions> {
    let opts = ConnectFireOptions::parse();

    if opts.verbose {
        info!(target:"startup", "{}", opts);
    }

    Ok(opts)
}

/*-------------------------------------------------------------------------------------------------
 *                                    Stats for this run.
 *-----------------------------------------------------------------------------------------------*/
struct FireStats {
    longest: Option<Fire>,
    most_powerful: Option<Fire>,
    hottest: Option<Fire>,
    sat: Satellite,
    count: usize,
    max_active: usize,
}

impl Display for FireStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, " ---- Summary Stats for Connect Fire {} ----", self.sat)?;
        writeln!(f, "\n           Processed {:9} Fires", self.count)?;
        writeln!(
            f,
            "      Maximum Number of Active Fires {:9}\n",
            self.max_active
        )?;
        if let Some(ref longest) = self.longest {
            writeln!(f, "   -- Longest Duration Fire --")?;
            writeln!(f, "{}", longest)?;
        } else {
            writeln!(f, "No longest duration fire for stats.")?;
        }

        if let Some(ref most_powerful) = self.most_powerful {
            writeln!(f, "   -- Most Powerful Fire --")?;
            writeln!(f, "{}", most_powerful)?;
        } else {
            writeln!(f, "No most powerful fire for stats.")?;
        }

        if let Some(ref hottest) = self.hottest {
            writeln!(f, "   -- Hottest Fire --")?;
            writeln!(f, "{}", hottest)?;
        } else {
            writeln!(f, "No hottest fire for stats.")?;
        }

        Ok(())
    }
}

impl FireStats {
    fn new(sat: Satellite) -> Self {
        FireStats {
            longest: None,
            most_powerful: None,
            hottest: None,
            sat,
            count: 0,
            max_active: 0,
        }
    }

    fn update(&mut self, fires: &FireList) {
        // Return early if the list is empty. There's nothing to do.
        if fires.is_empty() {
            return;
        }

        //
        // Get the maximums for the currentl list.
        //
        let mut fires_longest_dur = Duration::minutes(0);
        let mut fires_longest: Option<&Fire> = None;

        let mut fires_most_power_power = -f64::INFINITY;
        let mut fires_most_power: Option<&Fire> = None;

        let mut fires_hottest_temp = -f64::INFINITY;
        let mut fires_hottest: Option<&Fire> = None;

        for fire in fires.iter() {
            if fire.duration() > fires_longest_dur {
                fires_longest_dur = fire.duration();
                fires_longest = Some(fire);
            }

            if fire.max_power() > fires_most_power_power {
                fires_most_power_power = fire.max_power();
                fires_most_power = Some(fire);
            }

            if fire.max_temperature() > fires_hottest_temp {
                fires_hottest_temp = fire.max_temperature();
                fires_hottest = Some(fire);
            }

            self.count += 1;
            self.max_active = self.max_active.max(fires.len());
        }

        if let Some(fires_longest) = fires_longest {
            if let Some(ref mut longest) = self.longest {
                if fires_longest_dur > longest.duration() {
                    *longest = fires_longest.clone();
                }
            } else {
                self.longest = Some(fires_longest.clone());
            }
        }

        if let Some(fires_most_power) = fires_most_power {
            if let Some(ref mut most_powerful) = self.most_powerful {
                if fires_most_power_power > most_powerful.max_power() {
                    *most_powerful = fires_most_power.clone();
                }
            } else {
                self.most_powerful = Some(fires_most_power.clone());
            }
        }

        if let Some(fires_hottest) = fires_hottest {
            if let Some(ref mut hottest) = self.hottest {
                if fires_hottest_temp > hottest.max_power() {
                    *hottest = fires_hottest.clone();
                }
            } else {
                self.hottest = Some(fires_hottest.clone());
            }
        }
    }
}

/*-------------------------------------------------------------------------------------------------
 *                                   Processing For A Satellite
 *-----------------------------------------------------------------------------------------------*/
fn process_rows_for_satellite<P1: AsRef<Path>, P2: AsRef<Path>, P3: AsRef<Path>>(
    fires_db_store: P1,
    clusters_db_store: P2,
    sat: Satellite,
    area: BoundingBox,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    kml_path: P3,
    to_db_filler: Sender<DatabaseMessage>,
    verbose: bool,
) -> SatFireResult<()> {
    let db = FiresDatabase::connect(fires_db_store.as_ref())?;

    let mut current_fires = db.ongoing_fires(sat)?;

    let start = match (start, db.last_observed(sat)) {
        (Some(start), None) => start,
        (None, Some(last_observed)) => last_observed,
        (Some(start), Some(last_observed)) => {
            if last_observed < start {
                panic!(
                    "Database already started before but not complete up to: {}",
                    start
                );
            }

            if last_observed > start {
                last_observed
            } else {
                start
            }
        }
        (None, None) => sat.operational(),
    };
    let end = end.unwrap_or_else(|| Utc::now());

    drop(db);

    if verbose {
        info!(target: sat.name(), "Using start time of {}", start);
        info!(target: sat.name(), "Using end time of {}", end);
        info!(target: sat.name(), "Using bounding box of {}", area);
        info!(target: sat.name(), "Retrieved {} ongoing fires.", current_fires.len());
    }

    let mut new_fires = FireList::new();
    let mut old_fires = FireList::new();

    let db = ClusterDatabase::connect(clusters_db_store.as_ref())?;
    let mut stats = FireStats::new(sat);

    let mut rows = db.query_clusters(Some(sat), None, start, end, area)?;
    let rows = rows.rows()?;

    let mut current_time_step: DateTime<Utc> =
        DateTime::from_utc(NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0), Utc);
    let mut last_merge = current_time_step;

    let mut num_absorbed = 0;
    let mut num_new = 0;
    let current_group = vec![];
    for (group_time, group) in rows
        .map(|cluster| cluster.expect("Database error getting row."))
        .scan(current_group, |group, cluster| {
            let start = cluster.start;

            if start != current_time_step {
                let mut next_group = Vec::with_capacity(group.capacity());
                std::mem::swap(&mut next_group, group);

                let group_time = current_time_step;
                current_time_step = start;

                Some(Some((group_time, next_group)))
            } else {
                group.push(cluster);
                Some(None)
            }
        })
        .filter_map(|val| val)
    {
        if group_time - last_merge > Duration::hours(1) {
            // Only merge once per hour to speed things up.
            let num_merged = current_fires.merge_fires(&mut old_fires);
            let num_old = current_fires.drain_stale_fires(&mut old_fires, group_time);
            last_merge = group_time;

            if verbose {
                info!(target: sat.name(), "{:>23}, {:>8}, {:>6}, {:>4}, {:>4}, {:>6}",
                    group_time, num_absorbed, num_merged, num_old, num_new, current_fires.len());
            }

            num_absorbed = 0;
            num_new = 0;

            to_db_filler
                .send(DatabaseMessage::Fires(std::mem::take(&mut old_fires)))
                .expect("Error sending Fires message to database:");

            if SHUT_DOWN.load(Ordering::SeqCst) {
                warn!(target: sat.name(), "Shutting down early.");
                break;
            }
        }

        num_new += current_fires.extend(&mut new_fires);

        stats.update(&current_fires);

        if let Some(mut view) = FireListView::new(&mut current_fires) {
            for cluster in group {
                let clusterid = cluster.rowid;
                let fireid = match view.update(cluster) {
                    FireListUpdateResult::NoMatch(cluster) => {
                        let fireid = NEXT_WILDFIRE_ID.fetch_add(1, Ordering::SeqCst);
                        new_fires.create_add_fire(fireid, cluster);
                        fireid
                    }
                    FireListUpdateResult::Match(fireid) => {
                        num_absorbed += 1;
                        fireid
                    }
                };

                match to_db_filler.send(DatabaseMessage::Association((fireid, clusterid))) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Error sending Association message to database: {}", err);
                        return Err("Unable to send to_db_filler".into());
                    }
                }
            }
        } else {
            for cluster in group {
                let clusterid = cluster.rowid;
                let fireid = NEXT_WILDFIRE_ID.fetch_add(1, Ordering::SeqCst);
                new_fires.create_add_fire(fireid, cluster);

                match to_db_filler.send(DatabaseMessage::Association((fireid, clusterid))) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Error sending Association message to database: {}", err);
                        return Err("Unable to send to_db_filler".into());
                    }
                }
            }
        }
    }

    let num_merged = current_fires.merge_fires(&mut old_fires);
    let num_old = current_fires.drain_stale_fires(&mut old_fires, current_time_step);
    let num_new = current_fires.extend(&mut new_fires);

    current_fires.save_kml(Duration::days(1), kml_path)?;

    if verbose {
        info!(target: sat.name(), "{:>23}, {:>8}, {:>6}, {:>4}, {:>4}, {:>6}",
            current_time_step, num_absorbed, num_merged, num_old, num_new, current_fires.len());
    }

    old_fires.extend(&mut current_fires);
    stats.update(&old_fires);

    assert!(current_fires.is_empty());
    assert!(new_fires.is_empty());

    to_db_filler
        .send(DatabaseMessage::Fires(old_fires))
        .map_err(|_| "Undable to send to_db_filler".to_owned())?;

    if verbose {
        info!(target: "stats", "{}", stats);
    }

    Ok(())
}

/*-------------------------------------------------------------------------------------------------
 *                                 A thread for filling the database.
 *-----------------------------------------------------------------------------------------------*/
enum DatabaseMessage {
    Fires(FireList),
    Association((u64, u64)),
}

fn database_filler(
    db_store: PathBuf,
    messages: Receiver<DatabaseMessage>,
) -> JoinHandle<SatFireResult<()>> {
    thread::spawn(move || {
        let db = FiresDatabase::connect(db_store)?;
        let mut add_fire = db.prepare_to_add_fires()?;

        for message in messages {
            match message {
                DatabaseMessage::Fires(fires) => add_fire.add_fires(&fires)?,
                DatabaseMessage::Association((fireid, clusterid)) => {
                    add_fire.add_association(fireid, clusterid)
                }
            }
        }

        Ok(())
    })
}

/*-------------------------------------------------------------------------------------------------
 *                                       Signal Handlers
 *-----------------------------------------------------------------------------------------------*/
fn register_signal_handlers() {
    unsafe {
        libc::signal(libc::SIGTERM, handle_shutdown_signal as usize);
        libc::signal(libc::SIGQUIT, handle_shutdown_signal as usize);
        libc::signal(libc::SIGINT, handle_shutdown_signal as usize);
    }
}

fn handle_shutdown_signal(_signal: libc::c_int) {
    register_signal_handlers();

    SHUT_DOWN.store(true, Ordering::SeqCst);
}

/*-------------------------------------------------------------------------------------------------
 *                                             Main
 *-----------------------------------------------------------------------------------------------*/
fn main() -> SatFireResult<()> {
    register_signal_handlers();

    SimpleLogger::new().init()?;

    let opts = parse_args()?;

    FiresDatabase::initialize(&opts.fires_store_file)?;
    let fires_db = FiresDatabase::connect(&opts.fires_store_file)?;
    let next_id = fires_db.next_wildfire_id()?;
    NEXT_WILDFIRE_ID.store(next_id, Ordering::SeqCst);
    drop(fires_db);

    if opts.verbose {
        info!(target: "startup", "Next fire ID {}", next_id);
    }

    let (send_to_db_filler, from_processing) = bounded(1024);

    let mut jh_processing = Vec::with_capacity(Satellite::iter().count());

    if opts.verbose {
        info!(target: "startup", "{:>23}, {:>8}, {:>6}, {:>4}, {:>4}, {:>6}",
            "scan start time", "Absorbed", "Merged", "Old", "New", "Active");
    }

    for sat in Satellite::iter() {
        let mut kml_path = opts.clusters_store_file.clone();
        kml_path.set_file_name(sat.name());
        kml_path.set_extension("kml");
        let clusters_store_file = opts.clusters_store_file.clone();
        let fires_store_file = opts.fires_store_file.clone();
        let send_to_db_filler = send_to_db_filler.clone();

        let jh = std::thread::spawn(move || {
            process_rows_for_satellite(
                fires_store_file,
                clusters_store_file,
                sat,
                opts.bbox,
                opts.start,
                opts.end,
                kml_path,
                send_to_db_filler,
                opts.verbose,
            )
        });

        jh_processing.push(jh);
    }
    drop(send_to_db_filler);

    let jh_db_filler = database_filler(opts.fires_store_file, from_processing);

    jh_db_filler
        .join()
        .expect("Error joining the database filler thread.")?;

    for jh in jh_processing {
        jh.join().expect("Error joining a processing thread.")?;
    }

    Ok(())
}
