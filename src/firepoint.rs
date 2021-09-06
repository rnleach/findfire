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
pub struct FirePoint {
    /// The x-coordinate (column number, often indexed as 'i') in the grid.
    pub x: isize,
    /// The y-coordinate (row number, often indexed as 'j') in the grid.
    pub y: isize,
    /// The latitudes of the corners of the area
    pub lats: [f64; 4],
    /// The longitudes of the corners of the area
    pub lons: [f64; 4],
    /// The power of the fire in that pixel in megawatts.
    pub power: f64,
}
