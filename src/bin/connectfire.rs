use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Parser;
use satfire::{
    BoundingBox, Coord, Fire, FireDatabase, FireList, Geo, KmlFile, SatFireResult, Satellite,
};
use std::{
    fmt::{self, Display, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use strum::IntoEnumIterator;

/*-------------------------------------------------------------------------------------------------
 *                                        Global State
 *-----------------------------------------------------------------------------------------------*/
static NEXT_WILDFIRE_ID: AtomicU64 = AtomicU64::new(0);

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
    /// The path to the database file.
    ///
    /// If this is not specified, then the program will check for it in the "CLUSTER_DB"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "CLUSTER_DB")]
    store_file: PathBuf,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

impl Display for ConnectFireOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "\n")?; // yes, two blank lines.
        writeln!(f, "    Database: {}", self.store_file.display())?;
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
        println!("{}", opts);
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
}

impl Display for FireStats {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, " -- Summary Stats for Connect Fire {} --", self.sat)?;
        if let Some(ref longest) = self.longest {
            writeln!(f, " -- Longest Duration Fire --")?;
            writeln!(f, "{}", longest)?;
        } else {
            writeln!(f, "No longest duration fire for stats.")?;
        }

        if let Some(ref most_powerful) = self.most_powerful {
            writeln!(f, " -- Most Powerful Fire --")?;
            writeln!(f, "{}", most_powerful)?;
        } else {
            writeln!(f, "No most powerful fire for stats.")?;
        }

        if let Some(ref hottest) = self.hottest {
            writeln!(f, " -- Hottest Fire --")?;
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
 *                                   Output List of Fires as KML
 *-----------------------------------------------------------------------------------------------*/
fn save_wildfire_list_as_kml<P: AsRef<Path>>(kml_path: P, fires: &FireList) -> SatFireResult<()> {
    let mut kml = KmlFile::start_document(kml_path)?;

    kml.start_style(Some("fire"))?;
    kml.create_poly_style(Some("880000FF"), true, false)?;
    kml.create_icon_style(
        Some("http://maps.google.com/mapfiles/kml/shapes/firedept.png"),
        1.3,
    )?;
    kml.finish_style()?;

    let mut name = String::new();
    let mut description = String::new();
    for fire in fires.iter() {
        name.clear();
        let _ = write!(&mut name, "{}", fire.id());

        kml.start_folder(Some(&name), None, false)?;

        description.clear();
        let _ = write!(
            &mut description,
            concat!(
                "ID: {}<br/>",
                "Start: {}<br/>",
                "End: {}<br/>",
                "Duration: {}<br/>",
                "Max Power: {:.0} MW<br/>",
                "Max Temperature: {:.0} Kelvin<br/>",
            ),
            fire.id(),
            fire.first_observed(),
            fire.last_observed(),
            fire.duration(),
            fire.max_power(),
            fire.max_temperature()
        );

        kml.start_placemark(Some(&name), Some(&description), Some("#fire"))?;
        let centroid = fire.centroid();
        kml.create_point(centroid.lat, centroid.lon, 0.0)?;
        kml.finish_placemark()?;

        fire.pixels().kml_write(&mut kml);
        kml.finish_folder()?;
    }

    Ok(())
}
/*-------------------------------------------------------------------------------------------------
 *                                   Processing For A Satellite
 *-----------------------------------------------------------------------------------------------*/
fn process_rows_for_satellite<P: AsRef<Path>>(
    db: &FireDatabase,
    sat: Satellite,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    area: BoundingBox,
    kml_path: P,
    verbose: bool,
) -> SatFireResult<()> {
    let mut stats = FireStats::new(sat);

    let mut rows = db.query_clusters(Some(sat), None, start, end, area)?;
    let rows = rows.rows()?;

    let mut current_fires = FireList::new(); // TODO: Load these from the database.
    let mut new_fires = FireList::new();
    let mut old_fires = FireList::new();

    let mut current_time_step: DateTime<Utc> =
        DateTime::from_utc(NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0), Utc);

    let mut num_absorbed = 0;
    for cluster in rows {
        let cluster = cluster?;

        let start = cluster.start;

        if start != current_time_step {
            let num_merged = current_fires.merge_fires(&mut old_fires);
            let num_old = current_fires.drain_stale_fires(&mut old_fires, current_time_step);
            let num_new = current_fires.extend(&mut new_fires);

            if verbose {
                println!(
                    "Absorbed = {:4} Merged = {:4} Aged out = {:4} New = {:4} at {}",
                    num_absorbed, num_merged, num_old, num_new, current_time_step
                );
            }

            current_time_step = start;
            num_absorbed = 0;

            stats.update(&current_fires);

            // TODO: send old fires to a database thread.
        }

        if let Some(cluster) = current_fires.update(cluster) {
            let id = NEXT_WILDFIRE_ID.fetch_add(1, Ordering::SeqCst);
            new_fires.create_add_fire(id, cluster);
        } else {
            num_absorbed += 1;
        }
    }

    let num_merged = current_fires.merge_fires(&mut old_fires);
    let num_old = current_fires.drain_stale_fires(&mut old_fires, current_time_step);
    let num_new = current_fires.extend(&mut new_fires);

    save_wildfire_list_as_kml(kml_path, &current_fires)?;

    if verbose {
        println!(
            "Absorbed = {:4} Merged = {:4} Aged out = {:4} New = {:4} at {}",
            num_absorbed, num_merged, num_old, num_new, current_time_step
        );
    }

    old_fires.extend(&mut current_fires);
    stats.update(&old_fires);

    assert!(current_fires.is_empty());
    assert!(new_fires.is_empty());

    // TODO: send old fires to a database thread

    if verbose {
        println!("{}", stats);
    }

    Ok(())
}

/*-------------------------------------------------------------------------------------------------
 *                                             Main
 *-----------------------------------------------------------------------------------------------*/
fn main() -> SatFireResult<()> {
    let opts = parse_args()?;

    let db = FireDatabase::connect(&opts.store_file)?;

    let start = DateTime::from_utc(NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0), Utc);
    let end = Utc::now();

    let area = BoundingBox {
        ll: Coord {
            lat: -90.0,
            lon: -180.0,
        },
        ur: Coord {
            lat: 90.0,
            lon: 180.0,
        },
    };

    let next_id = db.next_wildfire_id()?;
    NEXT_WILDFIRE_ID.store(next_id, Ordering::SeqCst);

    if opts.verbose {
        println!("Next fire ID {}", next_id);
    }

    for sat in Satellite::iter() {
        let mut kml_path = opts.store_file.clone();
        kml_path.set_file_name(sat.name());
        kml_path.set_extension("kml");
        process_rows_for_satellite(&db, sat, start, end, area, &kml_path, opts.verbose)?;
    }

    Ok(())
}
