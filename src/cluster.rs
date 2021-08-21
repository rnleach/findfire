/*!
 * Types and functions for working with clusters.
 *
 * A cluster describes the aggregate properties of a connected group (or cluster) of FirePoint
 * objects.
 */

/**
 * The aggregate properties of a connected group of FirePoint objects.
 */
struct Cluster {
    /// The row id in the database. If this is 0 or less, the row is not yet known.
    rowid: i64,
    /// Average latitude of the points in the cluster.
    lat: f64,
    /// Average longitude of the points in the cluster.
    lon: f64,
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    power: f64,
    /// The distance from the cluster center to the farthest point in the cluster.
    radius: f64,
    /// The number of points that are in this cluster.
    count: u32,
}

/**
 * Keep a cluster list with metadata about the file it was derived from.
 *
 * If there is an error, the error member will be true, there will be an error message, and the
 * clusters pointer will be set to null.
 */
struct ClusterList {
    /// This is the sector, "FDCC", "FDCF", or "FDCM"
    ///
    /// FDCC is the CONUS scale
    /// FDCF is the full disk scale
    /// FDCM is the mesosector scale
    sector: &'static str,
    /// This is the source satellite.
    ///
    /// At the time of writing it will either be "G16" or "G17"
    satellite: &'static str,
    /// Start time of the scan
    start: chrono::naive::NaiveDateTime,
    /// End time of the scan
    end: chrono::naive::NaiveDateTime,
    /// List of struct Cluster objects associated with the above metadata.
    clusters: Vec<Cluster>,
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
    pub fn from_fire_points(points: &[Cluster]) -> Self {
        unimplemented!()
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
    pub fn from_file<F: AsRef<std::path::Path>>(full_path: F) -> Self {
        unimplemented!()
    }
}

/**
 * Parse the file name and find the scan start time.
 */
pub fn find_start_time<F: AsRef<str>>(full_path: F) -> chrono::naive::NaiveDateTime {
    unimplemented!()
}
