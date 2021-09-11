use std::error::Error;

pub use crate::{Cluster, FireSatImage};

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
    /// Start time of the scan.
    pub scan_start: chrono::naive::NaiveDateTime,
    /// List of struct Cluster objects associated with the above metadata.
    pub clusters: Vec<Cluster>,
}

impl ClusterList {
    /**
     * Analyze a FireSatImage and return a ClusterList including the file metadata.
     *
     * #Arguments
     * fsat - the already loaded image data.
     */
    pub fn from_fire_sat_image(fsat: &FireSatImage) -> Result<Self, Box<dyn Error>> {
        let points = fsat.extract_fire_points()?;
        let clusters = Cluster::from_fire_points(points);

        let satellite = fsat.satellite();
        let sector = fsat.sector();
        let scan_start = fsat.start();

        Ok(ClusterList {
            satellite,
            sector,
            clusters,
            scan_start,
        })
    }
}
