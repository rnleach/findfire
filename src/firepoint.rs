/*!
 * All the data related to a point with fire detected.
 *
 * A FirePoint is a structure that holds all data associated with a pixel in the satellite imagery
 * that corresponds to a fire detection.
 */
use geo::{algorithm::winding_order::Winding, Coordinate, LineString, Polygon};

/**
 * Represents all the data associated with a single pixel in which the satellite has detected a
 * fire.
 */
#[derive(Debug, Clone, Copy, PartialEq)]
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

impl FirePoint {
    pub fn polygon(&self) -> Polygon<f64> {
        let mut poly: LineString<_> = self
            .lats
            .iter()
            .cloned()
            .zip(self.lons.iter().cloned())
            .map(|(lat, lon)| Coordinate { x: lon, y: lat })
            .collect();

        poly.close();

        debug_assert!(
            poly.is_ccw(),
            "Assert counter-clockwise winding failed: {:#?}",
            self
        );

        Polygon::new(poly, vec![])
    }
}
