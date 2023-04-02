// Turn off warnings temporarily
#![allow(dead_code)]

// Public API
pub use cluster::{Cluster, ClusterList};
pub use database::{
    ClusterDatabase, ClusterDatabaseAddCluster, ClusterDatabaseClusterRow,
    ClusterDatabaseQueryClusterPresent, ClusterDatabaseQueryClusters, FiresDatabase,
    FiresDatabaseAddFire, JointFiresClusterDatabases, JointQuerySingleFire,
};
pub use fire::{Fire, FireList, FireListUpdateResult, FireListView};
pub use geo::{BoundingBox, Coord, Geo};
pub use kml::{KmlFile, KmlWriter, KmzFile};
pub use pixel::{Pixel, PixelList};
pub use satellite::{
    parse_satellite_description_from_file_name, DataQualityFlagCode, MaskCode, Satellite, Sector,
};

/// A generic error type.
pub type SatFireError = Box<dyn Error + Send + Sync>;

/// A generic result type.
pub type SatFireResult<T> = Result<T, SatFireError>;

/// Parse the file name and find the scan start time.
pub fn start_time_from_file_name(fname: &str) -> Option<DateTime<Utc>> {
    let start_idx = fname.find("_s")? + 2;
    let slice = &fname[start_idx..];
    let slice = if slice.len() > 13 {
        &slice[..13]
    } else {
        return None;
    };

    NaiveDateTime::parse_from_str(slice, "%Y%j%H%M%S")
        .ok()
        .map(|naive| DateTime::<Utc>::from_utc(naive, Utc))
}

/// Parse the file name and find the scan end time.
pub fn end_time_from_file_name(fname: &str) -> Option<DateTime<Utc>> {
    let start_idx = fname.find("_e")? + 2;
    let slice = &fname[start_idx..];
    let slice = if slice.len() > 13 {
        &slice[..13]
    } else {
        return None;
    };

    NaiveDateTime::parse_from_str(slice, "%Y%j%H%M%S")
        .ok()
        .map(|naive| DateTime::<Utc>::from_utc(naive, Utc))
}

// Private API
mod cluster;
mod database;
mod fire;
mod firesatimage;
mod geo;
mod kml;
mod pixel;
mod satellite;

use chrono::{DateTime, NaiveDateTime, Utc};
use std::error::Error;

// test
#[cfg(test)]
mod test {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_start_time_from_file_name() {
        const CASE1: &str =
            "OR_ABI-L2-FDCF-M6_G17_s20212130100319_e20212130109386_c20212130109511.nc.zip";

        let case1_start = start_time_from_file_name(CASE1).unwrap();
        assert_eq!(
            case1_start,
            DateTime::<Utc>::from_utc(
                NaiveDate::from_yo_opt(2021, 213)
                    .and_then(|d| d.and_hms_opt(1, 0, 31))
                    .unwrap(),
                Utc
            )
        );

        let case1_end = end_time_from_file_name(CASE1).unwrap();
        assert_eq!(
            case1_end,
            DateTime::<Utc>::from_utc(
                NaiveDate::from_yo_opt(2021, 213)
                    .and_then(|d| d.and_hms_opt(1, 9, 38))
                    .unwrap(),
                Utc
            )
        );
    }
}
