//! Documentation for the binary is with the definition of `FindFireOptionsInit` below.

use chrono::{DateTime, Datelike, Timelike, Utc};
use clap::Parser;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, info, warn};
use satfire::{
    BoundingBox, Cluster, ClusterDatabase, ClusterList, Coord, Geo, KmlWriter, KmzFile,
    SatFireResult, Satellite, Sector,
};
use simple_logger::SimpleLogger;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
    thread::JoinHandle,
};
use strum::IntoEnumIterator;

/*-------------------------------------------------------------------------------------------------
 *                               Parse Command Line Arguments
 *-----------------------------------------------------------------------------------------------*/
///
/// Group individual satellite pixels showing fire into connected clusters.
///
/// This program walks a directory tree and analyzes all the NOAA Big Data files with GOES satellite
/// Fire Detection Characteristics (FDC) data. Individual pixels with fire detected are grouped into
/// clusters of adjacent pixels. The power and fire area are summed to get totals for the cluster,
/// and other statistics such as the maximum scanning angle (satellite perpsective) and the maximum
/// fire temperature and then the statistics and a geographic description of all the pixels in the
/// cluster are serialized and stored in a database. The source satellite,
/// scanning sector (Full Disk, CONUS, MesoSector), scan start, and scan end times are also stored
/// in the database with each cluster.
///
/// The goal of having all this data together is for other programs to read the data from the
/// database and perform more analysis.
///
/// This program queries an existing database to find if a file has been processed already before
/// processing it.
///
/// At the end of processing, some summary statistics are printed to the screen and a file called
/// findfire.kmz is output in the same location as the database file findfire.sqlite that has some
/// summary statistics about the clusters and images that were analyzed during this run.
///
#[derive(Debug, Parser)]
#[clap(bin_name = "findfire")]
#[clap(author, version, about)]
struct FindFireOptionsInit {
    /// The path to the cluster database file.
    ///
    /// If this is not specified, then the program will check for it in the "CLUSTER_DB"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "CLUSTER_DB")]
    cluster_store_file: PathBuf,

    /// The path to a KMZ file to produce from this run.
    ///
    /// If this is not specified, then the program will create on automatically by replacing the
    /// file extension on the store_file with "*.kmz".
    #[clap(short, long)]
    kmz_file: Option<PathBuf>,

    /// The path to the data directory that will be walked to find new data.
    ///
    /// If this is not specified, then the program will check for it in the "SAT_ARCHIVE"
    /// environment variable.
    #[clap(short, long)]
    #[clap(env = "SAT_ARCHIVE")]
    data_dir: PathBuf,

    /// Only look for data newer than the most recent in the database.
    #[clap(short, long)]
    new_only: bool,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

#[derive(Debug)]
struct FindFireOptionsChecked {
    /// The path to the database file.
    cluster_store_file: PathBuf,

    /// The path to a KMZ file to produce from this run.
    kmz_file: PathBuf,

    /// The path to the data directory that will be walked to find new data.
    data_dir: PathBuf,

    /// Only look for data newer than the most recent in the database.
    new_only: bool,

    /// Verbose output
    verbose: bool,
}

/// Get the command line arguments and check them.
///
/// If there is missing data, try to fill it in with environment variables.
fn parse_args() -> SatFireResult<FindFireOptionsChecked> {
    let FindFireOptionsInit {
        cluster_store_file,
        kmz_file,
        data_dir,
        new_only,
        verbose,
    } = FindFireOptionsInit::parse();

    let kmz_file = match kmz_file {
        Some(v) => v,
        None => {
            let mut clone = cluster_store_file.clone();
            clone.set_extension("kmz");
            clone
        }
    };

    Ok(FindFireOptionsChecked {
        cluster_store_file,
        kmz_file,
        data_dir,
        new_only,
        verbose,
    })
}

/*-------------------------------------------------------------------------------------------------
 *                                            Main
 *-----------------------------------------------------------------------------------------------*/
const NUM_LOADER_THREADS: u8 = 4;

fn main() -> SatFireResult<()> {
    SimpleLogger::new().init()?;

    let opts = parse_args()?;

    if opts.verbose {
        info!(target: "startup", "{:#?}", opts);
    }

    ClusterDatabase::initialize(&opts.cluster_store_file)?;

    let (to_present_filter, from_dir_walker) = bounded(512);
    let (to_loader, from_present_filter) = bounded(512);
    let (to_db_writer, from_loader) = bounded(512);

    let data_dir = &opts.data_dir;
    let store_file = &opts.cluster_store_file;
    let verbose = opts.verbose;
    let only_new = opts.new_only;

    let walk_dir = dir_walker(data_dir, store_file, to_present_filter, only_new, verbose)?;
    let filter_present = filter_already_processed(store_file, from_dir_walker, to_loader, verbose)?;
    let loader = loader_threads(from_present_filter, to_db_writer, verbose)?;
    let db_filler = db_filler_thread(
        &opts.cluster_store_file,
        from_loader,
        &opts.kmz_file,
        opts.verbose,
    )?;

    db_filler.join().expect("Error joining db filler thread")?;
    walk_dir.join().expect("Error joining dir walker thread")?;

    for jh in filter_present {
        jh.join().expect("Error joining filter thread")?;
    }

    for jh in loader {
        jh.join().expect("Error joining loader thread")?;
    }

    Ok(())
}

/*-------------------------------------------------------------------------------------------------
 *                           Threads - Functions that start threads
 *-----------------------------------------------------------------------------------------------*/
fn dir_walker<P: AsRef<Path>>(
    data_dir: P,
    store_file: P,
    to_db_present_filter: Sender<PathBuf>,
    only_new: bool,
    verbose: bool,
) -> SatFireResult<JoinHandle<SatFireResult<()>>> {
    let data_dir = data_dir.as_ref().to_path_buf();

    // Get the most recent version in the database if necessary
    let mut most_recent = HashMap::new();
    if only_new {
        let db = ClusterDatabase::connect(store_file)?;

        for sat in Satellite::iter() {
            let inner = most_recent.entry(sat).or_insert_with(HashMap::new);
            for sector in Sector::iter() {
                let latest = db
                    .newest_scan_start(sat, sector)
                    .unwrap_or_else(|_| sat.operational());
                inner.insert(sector, latest);

                if verbose {
                    info!(target: "startup", "Most Recent {} {}: {}", sat, sector, latest);
                }
            }
        }
    } else {
        for sat in Satellite::iter() {
            let inner = most_recent.entry(sat).or_insert_with(HashMap::new);
            for sector in Sector::iter() {
                inner.insert(sector, sat.operational());
            }
        }
    }

    let standard_dir_filter = create_standard_dir_filter(most_recent, verbose);

    let jh = std::thread::Builder::new()
        .name("findfire-walker".to_owned())
        .spawn(move || {
            for entry in walkdir::WalkDir::new(data_dir)
                .into_iter()
                .filter_entry(standard_dir_filter)
                // Skip errors silently
                .filter_map(|res| res.ok())
                // Only process directories and "*.nc" and "*.zip" files
                .filter(|e| {
                    // Pass if it is a directory, or it has the right extension
                    e.file_type().is_dir()
                        || e.path().extension().map(|ex| ex == "nc").unwrap_or(false)
                        || e.path().extension().map(|ex| ex == "zip").unwrap_or(false)
                })
            {
                to_db_present_filter.send(entry.into_path())?;
            }

            Ok(())
        })?;

    Ok(jh)
}

fn filter_already_processed<P: AsRef<Path>>(
    store_file: P,
    from_dir_walker: Receiver<PathBuf>,
    to_loader: Sender<PathBuf>,
    verbose: bool,
) -> SatFireResult<Vec<JoinHandle<SatFireResult<()>>>> {
    let store_file = store_file.as_ref().to_path_buf();

    let mut handles = Vec::with_capacity(num_cpus::get());

    for _ in 0..num_cpus::get() {
        let to_loader_clone = to_loader.clone();
        let from_dir_walker_clone = from_dir_walker.clone();
        let store_file_clone = store_file.clone();

        let jh = std::thread::Builder::new()
            .name("findifre-filter".to_owned())
            .spawn(move || {
                let db = ClusterDatabase::connect(store_file_clone)?;
                let mut is_present = db.prepare_to_query_clusters_present()?;

                for path in from_dir_walker_clone {
                    if let Some((sat, sector, start, end)) = path.file_name().and_then(|fname| {
                        satfire::parse_satellite_description_from_file_name(&fname.to_string_lossy())
                    }) {
                        if !is_present.present(sat, sector, start, end)? {
                            if verbose {
                                info!(target: "filter", "processing {} {} {}", sat, sector, start);
                                debug!(target: "filter", "processing {} {} {} - {}", sat, sector, start, path.display());
                            }

                            to_loader_clone.send(path)?;
                        } else if verbose {
                            info!(target: "filter", "already in db: {}", path.display());
                        }
                    }
                }
                Ok(())
            })?;

        handles.push(jh);
    }

    Ok(handles)
}

fn loader_threads(
    from_db_present_filter: Receiver<PathBuf>,
    to_db_writer: Sender<ClusterList>,
    verbose: bool,
) -> SatFireResult<Vec<JoinHandle<SatFireResult<()>>>> {
    let mut jhs = Vec::with_capacity(NUM_LOADER_THREADS as usize);

    for _ in 0..NUM_LOADER_THREADS {
        let from_db_present = from_db_present_filter.clone();
        let to_db_writer = to_db_writer.clone();

        let jh = std::thread::Builder::new()
            .name("findfire-load".to_owned())
            .spawn(move || {
                for path in from_db_present {
                    let mut clist = match ClusterList::from_file(&path) {
                        Ok(clist) => clist,
                        Err(err) => {
                            if verbose {
                                warn!(target: "loading", "({}) {}", err, path.display());
                            }

                            continue;
                        }
                    };

                    clist.filter(is_cluster_a_keeper);

                    to_db_writer.send(clist)?;
                }

                Ok(())
            })?;

        jhs.push(jh);
    }

    Ok(jhs)
}

fn db_filler_thread<P: AsRef<Path>>(
    store_file: P,
    from_loader: Receiver<ClusterList>,
    kmz_path: P,
    verbose: bool,
) -> SatFireResult<JoinHandle<SatFireResult<()>>> {
    let store_file = store_file.as_ref().to_path_buf();
    let kmz_path = kmz_path.as_ref().to_path_buf();

    let jh = std::thread::Builder::new()
        .name("findfire-dbase".to_owned())
        .spawn(move || {
            let bb = BoundingBox {
                ll: Coord {
                    lat: 24.0,
                    lon: -177.0,
                },
                ur: Coord {
                    lat: 90.0,
                    lon: -50.0,
                },
            };

            let db = ClusterDatabase::connect(store_file)?;
            let mut add_stmt = db.prepare_to_add_clusters()?;

            let mut cluster_stats: Option<ClusterStats> = None;
            let mut cluster_list_stats: Option<ClusterListStats> = None;

            for mut cluster_list in from_loader {
                cluster_list.filter_box(bb);
                ClusterStats::update(&mut cluster_stats, &cluster_list);
                ClusterListStats::update(&mut cluster_list_stats, &cluster_list);
                add_stmt.add(cluster_list)?;
            }

            if let (Some(ref cluster_stats), Some(ref cluster_list_stats)) =
                (cluster_stats, cluster_list_stats)
            {
                save_cluster_stats_kmz(kmz_path, cluster_stats)?;
                if verbose {
                    info!(target: "stats", "{}", cluster_stats);
                    info!(target: "stats", "{}", cluster_list_stats);
                }
            }

            Ok(())
        })?;

    Ok(jh)
}

/*-------------------------------------------------------------------------------------------------
 *                             Cluster and Image Statistics
 *-----------------------------------------------------------------------------------------------*/

/* Use this as the maximum value of the scan angle allowed for a cluster to be considered in the
 * summary statistics. This is a QC tool, there are a lot of outliers on the limb of the Earth as
 * viewed by the GOES satellites, and the angles / geometry seem to have something to do with it.
 *
 * The value of 8.3 degrees is based on visual inspection of a graph of cluster power vs max scan
 * angle of the cluster member centroids. Based on the satellite product documentation
 * (https://www.goes-r.gov/products/docs/PUG-L2+-vol5.pdf) I calculated that the limb of the Earth
 * is at a scan angle of about 8.7 degrees.
 */
const MAX_SCAN_ANGLE: f64 = 8.3;

#[derive(Debug, Clone)]
struct ClusterStat {
    fire: Cluster,
    sat: Satellite,
    sector: Sector,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl ClusterStat {
    fn update(
        &mut self,
        fire: &Cluster,
        sat: Satellite,
        sector: Sector,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) {
        self.fire = fire.clone();
        self.sat = sat;
        self.sector = sector;
        self.start = start;
        self.end = end;
    }
}

impl Display for ClusterStat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        let centroid = self.fire.centroid();

        writeln!(f, "      satellite: {}", self.sat.name())?;
        writeln!(f, "         sector: {}", self.sector.name())?;
        writeln!(f, "          start: {}", self.start)?;
        writeln!(f, "            end: {}", self.end)?;
        writeln!(f, "            Lat: {:10.6}", centroid.lat)?;
        writeln!(f, "            Lon: {:11.6}", centroid.lon)?;
        writeln!(f, " Max Scan Angle: {:3.0}", self.fire.max_scan_angle())?;
        writeln!(f, "          Count: {:3}", self.fire.pixel_count())?;
        writeln!(f, "          Power: {:5.0} MW", self.fire.total_power())?;
        writeln!(
            f,
            "           Area: {:5.0} square kilometers",
            self.fire.total_area()
        )?;
        writeln!(
            f,
            "Max Temperature: {:5.0} Kelvin",
            self.fire.max_temperature()
        )?;
        writeln!(f)
    }
}

#[derive(Debug, Clone)]
struct ClusterStats {
    biggest_fire: ClusterStat,
    hottest_fire: ClusterStat,

    num_clusters: u32,
    num_power_lt_1mw: u32,
    num_power_lt_10mw: u32,
    num_power_lt_100mw: u32,
    num_power_lt_1gw: u32,
    num_power_lt_10gw: u32,
    num_power_lt_100gw: u32,
}

impl ClusterStats {
    fn update(stats: &mut Option<Self>, clusters: &ClusterList) {
        let sat = clusters.satellite();
        let sector = clusters.sector();
        let start = clusters.scan_start();
        let end = clusters.scan_end();

        for cluster in clusters.clusters() {
            if cluster.max_scan_angle() >= MAX_SCAN_ANGLE {
                return;
            }

            if stats.is_none() {
                *stats = Some(ClusterStats {
                    biggest_fire: ClusterStat {
                        fire: cluster.clone(),
                        sat,
                        sector,
                        start,
                        end,
                    },
                    hottest_fire: ClusterStat {
                        fire: cluster.clone(),
                        sat,
                        sector,
                        start,
                        end,
                    },
                    num_power_lt_1mw: 0,
                    num_power_lt_10mw: 0,
                    num_power_lt_100mw: 0,
                    num_power_lt_1gw: 0,
                    num_power_lt_10gw: 0,
                    num_power_lt_100gw: 0,
                    num_clusters: 0,
                });
            }

            if let Some(stats) = stats {
                let power = cluster.total_power();
                if stats.biggest_fire.fire.total_power() < power {
                    ClusterStat::update(&mut stats.biggest_fire, cluster, sat, sector, start, end);
                }

                let max_temp = cluster.max_temperature();
                if stats.hottest_fire.fire.max_temperature() < max_temp {
                    ClusterStat::update(&mut stats.hottest_fire, cluster, sat, sector, start, end);
                }

                if power < 1.0 {
                    stats.num_power_lt_1mw += 1;
                }

                if power < 10.0 {
                    stats.num_power_lt_10mw += 1;
                }

                if power < 100.0 {
                    stats.num_power_lt_100mw += 1;
                }

                if power < 1_000.0 {
                    stats.num_power_lt_1gw += 1;
                }

                if power < 10_000.0 {
                    stats.num_power_lt_10gw += 1;
                }

                if power < 100_000.0 {
                    stats.num_power_lt_100gw += 1;
                }

                stats.num_clusters += 1;
            } else {
                unreachable!()
            }
        }
    }
}

fn u32_pct(num: u32, denom: u32) -> u32 {
    let num = num as f64;
    let denom = denom as f64;
    (num / denom * 100.0).round() as u32
}

impl Display for ClusterStats {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "\nIndividual Cluster Stats\n")?;
        writeln!(f, " Most Powerful:")?;
        write!(f, "{}", &self.biggest_fire)?;
        writeln!(f, "       Hottest:")?;
        write!(f, "{}", &self.hottest_fire)?;
        writeln!(f, "        Counts:")?;
        writeln!(f, "         Total: {:10}", self.num_clusters)?;
        writeln!(f, "Power <   1 MW: {:10}", self.num_power_lt_1mw)?;
        writeln!(f, "Power <  10 MW: {:10}", self.num_power_lt_10mw)?;
        writeln!(f, "Power < 100 MW: {:10}", self.num_power_lt_100mw)?;
        writeln!(f, "Power <   1 GW: {:10}", self.num_power_lt_1gw)?;
        writeln!(f, "Power <  10 GW: {:10}", self.num_power_lt_10gw)?;
        writeln!(f, "Power < 100 GW: {:10}", self.num_power_lt_100gw)?;
        writeln!(
            f,
            "  Pct <   1 MW: {:10}",
            u32_pct(self.num_power_lt_1mw, self.num_clusters)
        )?;
        writeln!(
            f,
            "  Pct <  10 MW: {:10}",
            u32_pct(self.num_power_lt_10mw, self.num_clusters)
        )?;
        writeln!(
            f,
            "  Pct < 100 MW: {:10}",
            u32_pct(self.num_power_lt_100mw, self.num_clusters)
        )?;
        writeln!(
            f,
            "  Pct <   1 GW: {:10}",
            u32_pct(self.num_power_lt_1gw, self.num_clusters)
        )?;
        writeln!(
            f,
            "  Pct <  10 GW: {:10}",
            u32_pct(self.num_power_lt_10gw, self.num_clusters)
        )?;
        writeln!(
            f,
            "  Pct < 100 GW: {:10}",
            u32_pct(self.num_power_lt_100gw, self.num_clusters)
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct ClusterListStat {
    sat: Satellite,
    sector: Sector,
    num_clusters: usize,
    total_power: f64,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl From<&ClusterList> for ClusterListStat {
    fn from(clusters: &ClusterList) -> Self {
        ClusterListStat {
            sat: clusters.satellite(),
            sector: clusters.sector(),
            num_clusters: clusters.len(),
            start: clusters.scan_start(),
            end: clusters.scan_end(),
            total_power: clusters.total_power(),
        }
    }
}

impl Display for ClusterListStat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "                satellite: {}", self.sat.name())?;
        writeln!(f, "                   sector: {}", self.sector.name())?;
        writeln!(f, "                    start: {}", self.start)?;
        writeln!(f, "                      end: {}", self.end)?;
        writeln!(f, "           Total Clusters: {}", self.num_clusters)?;
        writeln!(f, "              Total Power: {:.0} GW\n", self.total_power)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ClusterListStats {
    min_num: ClusterListStat,
    max_num: ClusterListStat,
    max_power: ClusterListStat,
    min_power: ClusterListStat,
}

impl ClusterListStats {
    fn update(stats: &mut Option<Self>, clusters: &ClusterList) {
        if let Some(stats) = stats.as_mut() {
            let num_clust = clusters.len();
            if num_clust > stats.max_num.num_clusters {
                stats.max_num = clusters.into();
            } else if num_clust < stats.min_num.num_clusters {
                stats.min_num = clusters.into();
            }

            let total_power = clusters.total_power();
            if total_power > stats.max_power.total_power {
                stats.max_power = clusters.into();
            }
            if total_power < stats.min_power.total_power {
                stats.min_power = clusters.into();
            }
        } else {
            *stats = Some(ClusterListStats {
                min_num: clusters.into(),
                max_num: clusters.into(),
                max_power: clusters.into(),
                min_power: clusters.into(),
            });
        }
    }
}

impl Display for ClusterListStats {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "    Max Image Power Stats:\n{}", self.max_power)?;
        writeln!(f, "    Min Image Power Stats:\n{}", self.min_power)?;
        writeln!(f, "Max Image Number Clusters:\n{}", self.max_num)?;
        writeln!(f, "Max Image Number Clusters:\n{}", self.min_num)?;

        Ok(())
    }
}

/*-------------------------------------------------------------------------------------------------
 *                         Filters for skipping files / directories / clusters
 *-----------------------------------------------------------------------------------------------*/
fn create_standard_dir_filter(
    most_recent_in_db: HashMap<Satellite, HashMap<Sector, DateTime<Utc>>>,
    verbose: bool,
) -> impl FnMut(&walkdir::DirEntry) -> bool {
    /* This filter assumes the data is stored in a directory tree like:
     *   SATELLITE/SECTOR/YEAR/DAY_OF_YEAR/HOUR/files
     *
     *   e.g.
     *   G16/ABI-L2-FDCF/2020/238/15/...files...
     */

    move |entry| -> bool {
        if entry.path().is_file() {
            // We're only concerned with trimming directories - at this point.
            true
        } else if entry.path().is_dir() {
            // Let's trim directories we KNOW have data that is too old
            let path = entry.path().to_string_lossy();

            // Get the satellite and sector. If we can't parse these, then we need to keep going
            // deeper.
            let sat = match Satellite::string_contains_satellite(&path) {
                Some(sat) => sat,
                None => return true,
            };

            let sector = match Sector::string_contains_sector(&path) {
                Some(sector) => sector,
                None => return true,
            };

            let most_recent = match most_recent_in_db.get(&sat) {
                Some(hm) => match hm.get(&sector) {
                    Some(mr) => *mr,
                    None => sat.operational(),
                },
                None => sat.operational(),
            };

            let mr_year = most_recent.year();
            let mr_doy = most_recent.ordinal() as i32;
            let mr_hour = most_recent.hour() as i32;

            let mut year = i32::MIN;
            let mut doy = i32::MIN;
            let mut hour = i32::MIN;

            for dir in entry.path().iter() {
                let sub_path = dir.to_string_lossy();

                if year == i32::MIN {
                    if sub_path.len() >= 4 {
                        // Try to parse the year
                        if let Ok(possible_year) = sub_path[..4].parse::<i32>() {
                            // If it's larger than 2016, it's probably the year.
                            if possible_year > 2016 {
                                year = possible_year;

                                // Return early if we can
                                match year {
                                    x if x < mr_year => {
                                        if verbose {
                                            info!(target:"directory-filter", "skipping {}", entry.path().display());
                                        }
                                        return false;
                                    }
                                    x if x > mr_year => {
                                        return true;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                } else if doy == i32::MIN {
                    if sub_path.len() >= 3 {
                        // Try to parse the day of the year
                        if let Ok(possible_doy) = sub_path[..3].parse::<i32>() {
                            // Limits on the day of the year
                            if possible_doy > 0 && possible_doy < 367 {
                                doy = possible_doy;

                                // Return early if we can
                                if year == mr_year && doy < mr_doy {
                                    if verbose {
                                        info!(target:"directory-filter", "skipping {}", entry.path().display());
                                    }
                                    return false;
                                } else if year == mr_year && doy > mr_doy {
                                    return true;
                                }
                            }
                        }
                    }
                } else if hour == i32::MIN {
                    // Collapsing the if statement breaks a pattern established above and is less
                    // clear. Quiet clippy!
                    #[allow(clippy::collapsible_if)]
                    if sub_path.len() >= 2 {
                        // Try to parse the hour of the day
                        if let Ok(possible_hour) = sub_path[..2].parse::<i32>() {
                            // Limits on hour of the day!
                            if (0..25).contains(&possible_hour) {
                                hour = possible_hour;

                                // We have all the info we need, we should be able to return
                                if year == mr_year && doy == mr_doy && hour < mr_hour {
                                    if verbose {
                                        info!(target:"directory-filter", "skipping {}", entry.path().display());
                                    }
                                    return false;
                                } else {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }

            // Not enough info, keep going!
            true
        } else {
            // If we can't tell, accept it for now
            true
        }
    }
}

fn is_cluster_a_keeper(cluster: &Cluster) -> bool {
    // Check if it meets our mask criteria
    let keep_mask_criteria = cluster.pixels().pixels().iter().any(|pixel| {
        match pixel.mask_flag.0 {
            10   // good_fire_pixel
            | 11 // saturated_fire_pixel
            | 12 // cloud_contaminated_fire_pixel
            | 13 // high_probability_fire_pixel
            | 14 // medium_probability_fire_pixel

            | 30 // temporally_filtered_good_fire_pixel
            | 31 // temporally_filtered_saturated_fire_pixel
            | 32 // temporally_filtered_cloud_contaminated_fire_pixel
            | 33 // temporally_filtered_high_probability_fire_pixel
            | 34 // temporally_filtered_medium_probablity_fire_pixel
                => {
                 true
            }
            _ => false
        }
    });

    let scan_angle_criteria = cluster.max_scan_angle() < MAX_SCAN_ANGLE;

    keep_mask_criteria && scan_angle_criteria
}

/*-------------------------------------------------------------------------------------------------
 *                             Save a Cluster in a KMZ File
 *-----------------------------------------------------------------------------------------------*/
fn save_cluster_stats_kmz<P: AsRef<Path>>(
    path: P,
    cluster_stats: &ClusterStats,
) -> SatFireResult<()> {
    let mut kmz = KmzFile::new(path)?;

    kmz.start_style(Some("fire"))?;
    kmz.create_poly_style(Some("880000FF"), true, true)?;
    kmz.create_icon_style(
        Some("http://maps.google.com/mapfiles/kml/shapes/firedept.png"),
        1.3,
    )?;
    kmz.finish_style()?;

    output_cluster_stat_kml(&mut kmz, "Biggest Fire", &cluster_stats.biggest_fire)?;
    output_cluster_stat_kml(&mut kmz, "Hottest Fire", &cluster_stats.hottest_fire)?;

    Ok(())
}

fn output_cluster_stat_kml<K: KmlWriter>(
    out: &mut K,
    label: &str,
    cluster: &ClusterStat,
) -> SatFireResult<()> {
    let description = format!(
        concat!(
            "Satellite: {}<br/>",
            "Sector: {}<br/>",
            "Power: {:.0} MW<br/>",
            "Area: {:.0} m^2<br/>",
            "Max Scan Angle: {:0.3}&deg;<br/>",
            "Max Temperature: {:.0}&deg;K"
        ),
        cluster.sat.name(),
        cluster.sector.name(),
        cluster.fire.total_power(),
        cluster.fire.total_area(),
        cluster.fire.max_scan_angle(),
        cluster.fire.max_temperature()
    );

    let centroid = cluster.fire.centroid();

    out.start_folder(Some(label), None, true)?;
    out.timespan(cluster.start, cluster.end)?;
    out.start_placemark(Some(label), Some(&description), Some("#fire"))?;
    out.create_point(centroid.lat, centroid.lon, 0.0)?;
    out.finish_placemark()?;

    cluster.fire.pixels().kml_write(out);

    out.finish_folder()?;

    Ok(())
}
