/*!
 * Types and functions for working with clusters.
 *
 * A cluster describes the aggregate properties of a connected group (or cluster) of FirePoint
 * objects.
 */
use crate::{error::FindFireError, firepoint::FirePoint, firesatimage::FireSatImage};

use std::{error::Error, path::Path};

/**
 * The aggregate properties of a connected group of FirePoint objects.
 */
#[derive(Clone, Copy, Debug)]
pub struct Cluster {
    /// The row id in the database. If this is 0 or less, the row is not yet known.
    pub rowid: i64,
    /// Average latitude of the points in the cluster.
    pub lat: f64,
    /// Average longitude of the points in the cluster.
    pub lon: f64,
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    pub power: f64,
    /// The distance from the cluster center to the farthest point in the cluster.
    pub radius: f64,
    /// The number of points that are in this cluster.
    pub count: i32,
}

impl Default for Cluster {
    fn default() -> Self {
        Cluster {
            rowid: -1,
            lat: f64::NAN,
            lon: f64::NAN,
            power: 0.0,
            radius: 0.0,
            count: 0,
        }
    }
}

/**
 * Keep a cluster list with metadata about the file it was derived from.
 *
 * If there is an error, the error member will be true, there will be an error message, and the
 * clusters pointer will be set to null.
 */
pub struct ClusterList {
    /// This is the sector, "FDCC", "FDCF", or "FDCM"
    ///
    /// FDCC is the CONUS scale
    /// FDCF is the full disk scale
    /// FDCM is the mesosector scale
    pub sector: &'static str,
    /// This is the source satellite.
    ///
    /// At the time of writing it will either be "G16" or "G17"
    pub satellite: &'static str,
    /// Start time of the scan
    pub start: chrono::naive::NaiveDateTime,
    /// End time of the scan
    pub end: chrono::naive::NaiveDateTime,
    /// List of struct Cluster objects associated with the above metadata.
    pub clusters: Vec<Cluster>,
}

impl Cluster {
    /**
     * Group FirePoint objects into clusters.
     *
     * FirePoint objects that are directly adjacent to each other are grouped into clusters where
     * each point is in direct contact with at least one other point in the cluster.
     *
     * #Arguments
     * points - an array of FirePoint objects.
     *
     * #Returns
     * An array of struct Cluster objects.
     */
    pub fn from_fire_points(mut points: Vec<FirePoint>) -> Vec<Self> {
        let mut clusters: Vec<Self> = vec![];
        let mut cluster_points: Vec<FirePoint> = vec![];

        const NULL_PT: FirePoint = FirePoint {
            x: 0,
            y: 0,
            power: f64::NAN,
            lat: f64::NAN,
            lon: f64::NAN,
        };

        for i in 0..points.len() {
            if points[i].x == 0 && points[i].y == 0 {
                continue;
            }

            let curr_pt = std::mem::replace(&mut points[i], NULL_PT);

            cluster_points.push(curr_pt);

            for j in (i + 1)..points.len() {
                if points[j].x == 0 && points[j].y == 0 {
                    continue;
                }

                let mut in_cluster = false;
                for a_point_in_cluster in &cluster_points {
                    let dx = (a_point_in_cluster.x - points[j].x).abs();
                    let dy = (a_point_in_cluster.y - points[j].y).abs();

                    if dx <= 1 && dy <= 1 {
                        in_cluster = true;
                        break;
                    }
                }

                if in_cluster {
                    let candidate = std::mem::replace(&mut points[j], NULL_PT);
                    cluster_points.push(candidate);
                }
            }

            let mut curr_clust = Cluster {
                count: 0,
                lat: 0.0,
                lon: 0.0,
                power: 0.0,
                radius: 0.0,
                rowid: 0,
            };

            for pnt in &cluster_points {
                curr_clust.lat += pnt.lat;
                curr_clust.lon += pnt.lon;
                curr_clust.power += pnt.power;
                curr_clust.count += 1;
            }

            curr_clust.lat /= curr_clust.count as f64;
            curr_clust.lon /= curr_clust.count as f64;

            for pnt in &cluster_points {
                let gs_distance = crate::geo::great_circle_distance(
                    pnt.lat,
                    pnt.lon,
                    curr_clust.lat,
                    curr_clust.lon,
                );

                curr_clust.radius = curr_clust.radius.max(gs_distance);
            }

            clusters.push(curr_clust);
            cluster_points.truncate(0);
        }

        clusters
    }
}

impl ClusterList {
    /**
     * Analyze a file and return a ClusterList including the file metadata.
     *
     * The metadata is gleaned from the file name at this time.
     *
     * #Arguments
     * full_path - the path to the file to analyze.
     */
    pub fn from_file<F: AsRef<std::path::Path>>(full_path: F) -> Result<Self, Box<dyn Error>> {
        let pth: &Path = full_path.as_ref();
        let fname = pth.file_name().unwrap().to_string_lossy();

        // Satellites
        const G16: &str = "G16";
        const G17: &str = "G17";

        let satellite = if fname.contains(G16) {
            G16
        } else if fname.contains(G17) {
            G17
        } else {
            return Err(Box::new(FindFireError {
                msg: "Invalid file name, no satellite description.",
            }));
        };

        // Sectors
        const CONUS: &str = "FDCC";
        const FULL_DISK: &str = "FDCF";
        const MESO: &str = "FDCM";

        let sector = if fname.contains(CONUS) {
            CONUS
        } else if fname.contains(FULL_DISK) {
            FULL_DISK
        } else if fname.contains(MESO) {
            MESO
        } else {
            return Err(Box::new(FindFireError {
                msg: "Invalid file name, no satellite sector description.",
            }));
        };

        let start = FireSatImage::find_start_time(&fname)?;
        let end = FireSatImage::find_end_time(&fname)?;

        let sat_data = FireSatImage::open(pth)?;
        let points = sat_data.extract_fire_points()?;
        let clusters = Cluster::from_fire_points(points);

        Ok(ClusterList {
            satellite,
            sector,
            clusters,
            start,
            end,
        })
    }
}
