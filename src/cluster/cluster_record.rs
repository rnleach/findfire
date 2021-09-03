use chrono::NaiveDateTime;

#[derive(Debug, Clone, Copy)]
pub struct ClusterRecord {
    /// Row id from the database.
    pub rowid: i64,
    /// The mid-point time of the scan this cluster was detected in.
    pub scan_time: NaiveDateTime,
    /// The average latitude of the points in this cluster.
    pub lat: f64,
    /// The average longitude of the points in this cluster.
    pub lon: f64,
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    pub power: f64,
    /// The distance from the cluster center to the farthest point in the cluster.
    pub radius: f64,
}

impl ClusterRecord {
    pub fn new(
        rowid: i64,
        scan_time: NaiveDateTime,
        lat: f64,
        lon: f64,
        power: f64,
        radius: f64,
    ) -> Self {
        ClusterRecord {
            rowid,
            scan_time,
            lat,
            lon,
            power,
            radius,
        }
    }
}
