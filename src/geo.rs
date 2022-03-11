//! Geographic primitives specifically suited to the needs of this crate.
use std::fmt::Display;

/// A coordinate consisting of a latitude and a longitude.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Coord {
    /// Latitude. Should be -90 to 90, but that's not checked or enforced.
    pub lat: f64,
    /// Longitude. Should be -180 to 180, but that's not checked or enforced.
    pub lon: f64,
}

impl Coord {
    /// Determine if these coordinates are close to each other.
    ///
    /// The eps parameter is the maximum distance between points in the same units as the
    /// coordinates that two points can have and still be considered close.
    pub fn is_close(&self, other: Coord, eps: f64) -> bool {
        let lat_diff = self.lat - other.lat;
        let lon_diff = self.lon - other.lon;
        let distance_squared = lat_diff * lat_diff + lon_diff * lon_diff;

        distance_squared <= (eps * eps)
    }
}

/// Represents a "square" area in latitude-longitude coordinates.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    /// The lower left corner of the box.  
    pub ll: Coord,
    /// The upper right corner of the box.
    pub ur: Coord,
}

impl Display for BoundingBox {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{},{},{},{}",
            self.ll.lat, self.ll.lon, self.ur.lat, self.ur.lon
        )
    }
}

impl BoundingBox {
    /// Check to see if a Coord is inside of a BoundingBox.
    ///
    /// # Arguments
    /// * `box` - is the bounding box in question.
    /// * `coord` - is the coordinate, or point, in question.
    /// * `eps` - is a fuzzy factor. Any point `eps` close to the box will be considered internal as
    ///    well. If `eps` is 0.0, then the comparison is exact.
    ///
    /// # Return
    ///
    /// Returns `true` if the point `coord` is interior to the `box`.
    pub fn contains_coord(&self, coord: Coord, eps: f64) -> bool {
        let lon_in_range = (coord.lon - self.ur.lon) < eps && (coord.lon - self.ll.lon) > -eps;
        let lat_in_range = (coord.lat - self.ur.lat) < eps && (coord.lat - self.ll.lat) > -eps;

        lon_in_range && lat_in_range
    }

    /// Check to see if these BoundingBox objects overlap.
    ///
    /// # Arguments
    /// * `other` - is the other box to compare to.
    /// * `eps` - is a fuzzy factor. In any point comparisons, any point within 'eps' close to the
    ///    box `self` will be considered as overlapping.
    pub fn overlap(&self, other: &BoundingBox, eps: f64) -> bool {
        let other_coords = [
            other.ll,
            other.ur,
            Coord {
                lat: other.ll.lat,
                lon: other.ur.lon,
            },
            Coord {
                lat: other.ur.lat,
                lon: other.ll.lon,
            },
        ];

        let self_coords = [
            self.ll,
            self.ur,
            Coord {
                lat: self.ll.lat,
                lon: self.ur.lon,
            },
            Coord {
                lat: self.ur.lat,
                lon: self.ll.lon,
            },
        ];

        for coord in other_coords {
            if self.contains_coord(coord, eps) {
                return true;
            }
        }

        for coord in self_coords {
            if other.contains_coord(coord, eps) {
                return true;
            }
        }

        false
    }
}

/// Some simple geographic operations.
pub trait Geo {
    /// Get the centroid of the object.
    ///
    /// This centroid is in the lat-lon space and does not consider that the euclidian distances
    /// are different at for a given difference in lat-lon near the equator vs near the poles.
    fn centroid(&self) -> Coord;

    /// Get the lat-lon bounding box for this object.
    fn bounding_box(&self) -> BoundingBox;
}

pub(crate) fn triangle_centroid(v1: Coord, v2: Coord, v3: Coord) -> Coord {
    let avg_lat = (v1.lat + v2.lat + v3.lat) / 3.0;
    let avg_lon = (v1.lon + v2.lon + v3.lon) / 3.0;

    Coord {
        lat: avg_lat,
        lon: avg_lon,
    }
}

/*-------------------------------------------------------------------------------------------------
 *                                    Helper types and functions
 *-----------------------------------------------------------------------------------------------*/

#[derive(Debug, Clone, Copy)]
pub(crate) struct Line {
    pub start: Coord,
    pub end: Coord,
}

impl Line {
    pub(crate) fn is_close(&self, coord: Coord, eps: f64) -> bool {
        let p0 = coord;
        let Line { start: p1, end: p2 } = self;
        let eps2 = eps * eps;

        let num = (p2.lon - p1.lon) * (p1.lat - p0.lat) - (p1.lon - p0.lon) * (p2.lat - p1.lat);
        let denom2 = (p2.lon - p1.lon) * (p2.lon - p1.lon) + (p2.lat - p1.lat) * (p2.lat - p1.lat);

        (num * num / denom2) <= eps2
    }

    pub fn intersect(&self, other: Line, eps: f64) -> Option<IntersectResult> {
        // Check if they are nearly co-linear
        let mut num_close = 0;
        if self.is_close(other.start, eps) {
            num_close += 1;
        }
        if self.is_close(other.end, eps) {
            num_close += 1;
        }
        if other.is_close(self.start, eps) {
            num_close += 1;
        }
        if other.is_close(self.end, eps) {
            num_close += 1;
        }
        if num_close > 1 {
            // Colinear.
            return None;
        }

        let m1 = (self.end.lat - self.start.lat) / (self.end.lon - self.start.lon);
        let m2 = (other.end.lat - other.start.lat) / (other.end.lon - other.start.lon);

        let x1 = self.start.lon;
        let y1 = self.start.lat;
        let x2 = other.start.lon;
        let y2 = other.start.lat;

        if m1 == m2 || (m1.is_infinite() && m2.is_infinite()) {
            // Parallel lines or colinear without matching end points.
            // NOTE: This also captures colinear cases.
            return None;
        }

        let x0;
        let y0;
        if m1.is_nan() {
            // self is a single point
            x0 = self.start.lon;
            y0 = self.start.lat;
        } else if m2.is_nan() {
            // other is a single point
            x0 = other.start.lon;
            y0 = other.start.lat;
        } else if m1.is_infinite() {
            // l1 is vertical
            x0 = self.start.lon;
            y0 = m2 * (x0 - x2) + y2;
        } else if m2.is_infinite() {
            // l2 is vertical
            x0 = other.start.lon;
            y0 = m1 * (x0 - x1) + y1;
        } else {
            x0 = (y2 - y1 + m1 * x1 - m2 * x2) / (m1 - m2);
            y0 = m1 * (x0 - x1) + y1;
        }

        debug_assert!(!x0.is_nan() && !y0.is_nan());

        let mut result = IntersectResult {
            intersection: Coord { lon: x0, lat: y0 },
            intersect_is_endpoints: false,
        };
        let intersect = result.intersection; // short-hand

        if y0 - self.start.lat.max(self.end.lat) > eps
            || self.start.lat.min(self.end.lat) - y0 > eps
            || x0 - self.start.lon.max(self.end.lon) > eps
            || self.start.lon.min(self.end.lon) - x0 > eps
        {
            // Test to make sure we are within the limits of self
            // In this case the intersection point lies outside the range of the self line segment.
            return None;
        } else if y0 - other.start.lat.max(other.end.lat) > eps
            || other.start.lat.min(other.end.lat) - y0 > eps
            || x0 - other.start.lon.max(other.end.lon) > eps
            || other.start.lon.min(other.end.lon) - x0 > eps
        {
            // Test to make sure we are within the limits of other
            // In this case the intersection point lies outside the range of the other line segment
            return None;
        } else {
            let is_self_endpoint =
                intersect.is_close(self.start, eps) || intersect.is_close(self.end, eps);

            let is_other_endpoint =
                intersect.is_close(other.start, eps) || intersect.is_close(other.end, eps);

            if is_self_endpoint && is_other_endpoint {
                result.intersect_is_endpoints = true;
            }
        }

        Some(result)
    }
}

pub(crate) struct IntersectResult {
    pub intersection: Coord,
    pub intersect_is_endpoints: bool,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_coord_are_close() {
        let left = Coord {
            lat: 45.5,
            lon: -120.0,
        };
        let right = Coord {
            lat: 45.5000002,
            lon: -120.0000002,
        };

        assert!(left.is_close(left, 1.0e-6));
        assert!(right.is_close(right, 1.0e-6));
        assert!(left.is_close(right, 1.0e-6));
        assert!(right.is_close(left, 1.0e-6));

        assert!(!left.is_close(right, 1.0e-8));
    }
}
