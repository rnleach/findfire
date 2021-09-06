use chrono::NaiveDateTime;
use geo::{Point, Polygon};

#[derive(Debug, Clone)]
pub struct ClusterRecord {
    /// Row id from the database.
    pub rowid: i64,
    /// The mid-point time of the scan this cluster was detected in.
    pub scan_time: NaiveDateTime,
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    pub power: f64,
    /// Perimeter
    pub perimeter: Polygon<f64>,
    /// Centroid
    pub centroid: Point<f64>,
}

impl ClusterRecord {
    pub fn new(
        rowid: i64,
        scan_time: NaiveDateTime,
        power: f64,
        perimeter: Polygon<f64>,
        centroid: Point<f64>,
    ) -> Self {
        ClusterRecord {
            rowid,
            scan_time,
            power,
            perimeter,
            centroid,
        }
    }
}
