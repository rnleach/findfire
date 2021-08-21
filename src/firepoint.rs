/*!
 * All the data related to a point with fire detected.
 *
 * A FirePoint is a structure that holds all data associated with a pixel in the satellite imagery
 * that corresponds to a fire detection.
 */

/**
 * Represents all the data associated with a single pixel in which the satellite has detected a
 * fire.
 */
struct FirePoint {
    /// The x-coordinate (column number, often indexed as 'i') in the grid.
    x: usize,
    /// The y-coordinate (row number, often indexed as 'j') in the grid.
    y: usize,
    /// The latitude
    lat: f64,
    /// The longitude
    lon: f64,
    /// The power of the fire in that pixel in megawatts.
    power: f64,
}

impl FirePoint {
    /**
     * Calculate the great circle distance between two FirePoint objects.
     *
     * Calculate the great circle distance between a and b.
     *
     * The distance between the points in kilometers.
     */
    pub fn great_circle_distance(&self, other: FirePoint) -> f64 {
        let a_lat = self.lat;
        let b_lat = other.lat;
        let a_lon = self.lon;
        let b_lon = other.lon;

        return crate::geo::great_circle_distance(a_lat, a_lon, b_lat, b_lon);
    }
}
