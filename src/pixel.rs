use crate::{
    geo::{BoundingBox, Coord, Geo},
    kml::KmlFile,
    satellite::{DataQualityFlagCode, MaskCode},
};
use std::io::{Read, Write};

/// The coordinates describing the area of a pixel viewed from a GOES satellite.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Pixel {
    /// The upper left (northwest) corner point of the pixel
    ul: Coord,
    /// The lower left (southwest) corner point of the pixel
    ll: Coord,
    /// The lower right (southeast) corner point of the pixel.
    lr: Coord,
    /// The upper right (northeast) corner point of the pixel.
    ur: Coord,
    /// The radiative power in MegaWatts in this pixel.
    power: f64,
    /// The estimated area of the pixel covered by the fire in square meters.
    area: f64,
    /// The estimated temperature of the fire in K
    temperature: f64,
    /// This is the scan angle as measured in the coordinate system of the satellite. The satellite
    /// measures the x and y positions of a pixel on a grid by the angle each makes with the central
    /// point which looks at nadir on the Earth. There are two values, an x scan angle and a y scan
    /// angle. They are combined via the Euclidian norm sqrt(x^2 + y^2) to form the scan_angle.
    ///
    /// Constant values of the scan angle form concentric circles around the nadir point on the
    /// Earth's surface. All points along that line have a very similar (equal if the Earth was a
    /// sphere) angle between the satellites view and the local zenith. This is a good proxy for
    /// how much of an edge on vs straight down view, which can be useful for quality control.
    scan_angle: f64,
    /// Mask is a code that describes the outcome of the algorithms that characterize a fire point.
    ///
    /// See the satfire_satellite_mask_code_to_string() function for reference.
    mask_flag: MaskCode,
    /// Data Quality Flag
    ///
    /// See the satfire_satellite_dqf_code_to_string() function for reference.
    data_quality_flag: DataQualityFlagCode,
}

impl Pixel {
    fn write_bytes<W: Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        let mut write_coord = |coord: &Coord| -> Result<(), std::io::Error> {
            w.write(&coord.lat.to_ne_bytes())?;
            w.write(&coord.lon.to_ne_bytes())?;
            Ok(())
        };

        write_coord(&self.ul)?;
        write_coord(&self.ll)?;
        write_coord(&self.lr)?;
        write_coord(&self.ur)?;

        w.write(&self.power.to_ne_bytes())?;
        w.write(&self.area.to_ne_bytes())?;
        w.write(&self.temperature.to_ne_bytes())?;
        w.write(&self.scan_angle.to_ne_bytes())?;
        w.write(&self.mask_flag.0.to_ne_bytes())?;
        w.write(&self.data_quality_flag.0.to_ne_bytes())?;

        // Add some padding to match the old binary format from C
        const PADDING: u32 = 0;
        w.write(&PADDING.to_ne_bytes())?;

        Ok(())
    }

    fn read_bytes<R: Read>(r: &mut R) -> Self {
        let mut read_coord = || -> Coord {
            let mut buf: [u8; 8] = [0; 8];
            r.read_exact(&mut buf).unwrap();
            let lat = f64::from_ne_bytes(buf);
            r.read_exact(&mut buf).unwrap();
            let lon = f64::from_ne_bytes(buf);
            Coord { lat, lon }
        };

        let ul = read_coord();
        let ll = read_coord();
        let lr = read_coord();
        let ur = read_coord();

        let mut buf: [u8; 8] = [0; 8];
        r.read_exact(&mut buf).unwrap();
        let power = f64::from_ne_bytes(buf);
        r.read_exact(&mut buf).unwrap();
        let area = f64::from_ne_bytes(buf);
        r.read_exact(&mut buf).unwrap();
        let temperature = f64::from_ne_bytes(buf);
        r.read_exact(&mut buf).unwrap();
        let scan_angle = f64::from_ne_bytes(buf);

        let mut buf: [u8; 2] = [0; 2];
        r.read_exact(&mut buf).unwrap();
        let mask_flag = MaskCode(i16::from_ne_bytes(buf));
        r.read_exact(&mut buf).unwrap();
        let data_quality_flag = DataQualityFlagCode(i16::from_ne_bytes(buf));

        // Read out the padding to match the old C binary format
        let mut buf: [u8; 4] = [0; 4];
        r.read_exact(&mut buf).unwrap();

        Pixel {
            ul,
            ll,
            lr,
            ur,
            power,
            area,
            temperature,
            scan_angle,
            mask_flag,
            data_quality_flag,
        }
    }
}

impl Geo for Pixel {
    /// Calculate the centroid of a Pixel.
    ///
    /// This function uses an algorithm that assumes the pixel is a quadrilateral, which is enforced
    /// by the definition of the Pixel type.
    #[rustfmt::skip]
    fn centroid(&self) -> Coord {
        /* Steps to calculatule the centroid of a quadrilateral.
         *
         *  1) Break the quadrilateral into two triangles by creating a diagonal.
         *  2) Calculate the centroid of each triangle by taking the average of it's 3 Coords
         *  3) Create a line connecting the centroids of each triangle.
         *  4) Repeat the process by creating the other diagonal.
         *  5) Find the intersection of the two resulting lines, that is the centroid of the
         *     quadrilateral.
         */
        use crate::geo::{triangle_centroid, Line};

        let t1_c = triangle_centroid(self.ul, self.ll, self.lr);
        let t2_c = triangle_centroid(self.ul, self.ur, self.lr);
        let diag1_centroids = Line {start: t1_c, end: t2_c};

        let t3_c = triangle_centroid(self.ul, self.ll, self.ur);
        let t4_c = triangle_centroid(self.lr, self.ur, self.ll);
        let diag2_centroids = Line {start: t3_c, end: t4_c};

        let res = diag1_centroids.intersect(diag2_centroids, 1.0e-30).unwrap();

        res.intersection
    }

    #[rustfmt::skip]
    fn bounding_box(&self) -> BoundingBox {
        let min_lat = self.ll.lat.min(self.lr.lat).min(self.ul.lat).min(self.ur.lat);
        let max_lat = self.ll.lat.max(self.lr.lat).max(self.ul.lat).max(self.ur.lat);
        let min_lon = self.ll.lon.min(self.lr.lon).min(self.ul.lon).min(self.ur.lon);
        let max_lon = self.ll.lon.max(self.lr.lon).max(self.ul.lon).max(self.ur.lon);

        let ll = Coord {lat: min_lat, lon: min_lon};
        let ur = Coord {lat: max_lat, lon: max_lon};

        BoundingBox { ll, ur }
    }
}

impl Pixel {
    /// Tests if these pixels are basically the same pixel in a geographic sense.
    ///
    /// This only compares the corners of the pixels and not other properties such as power, fire
    /// area, or temperature.
    pub fn approx_equal(&self, other: &Pixel, eps: f64) -> bool {
        self.ul.is_close(other.ul, eps)
            && self.ur.is_close(other.ur, eps)
            && self.lr.is_close(other.lr, eps)
            && self.ll.is_close(other.ll, eps)
    }

    /// Determine if a coordinate is interior to a pixel.
    ///
    /// Interior means that it is NOT on the boundary. The eps parameter is used by an interanl line
    /// intersection function to detect if the intersection point is very close to an end point.
    ///
    #[rustfmt::skip]
    pub fn contains_coord(&self, coord: Coord, eps: f64) -> bool {
        use crate::geo::Line;

        // Check if it's outside the bounding box first. This is easy, and if it is,
        // then we already know the answer.
        if !self.bounding_box().contains_coord(coord, eps) {
            return false;
        }

        // Make a line from the point in question to each corner of the quadrilateral. If any of those
        // lines intersect an edge of the quadrilateral, then the point is outside. Note that the
        // line intersection function takes the eps argument and uses that to determine if the
        // intersection is near an end point. If it is, then we ignore it. So there is some
        // fuzziness to this function. If a coordinate outside the pixel is close enough to one of
        // the edges, it is possible it would be classified as inside. But it has to be eps close!
        // And even then it's not guaranteed.
        let pxl_lines = [
            Line {start: self.ul, end: self.ur},
            Line {start: self.ur, end: self.lr},
            Line {start: self.lr, end: self.ll},
            Line {start: self.ll, end: self.ul},
        ];

        let coord_lines = [
            Line {start: coord, end: self.ul},
            Line {start: coord, end: self.ur},
            Line {start: coord, end: self.ll},
            Line {start: coord, end: self.lr},
        ];

        for p_line in pxl_lines {
            for c_line in coord_lines {
                if let Some(res) = p_line.intersect(c_line, eps) {
                    if !res.intersect_is_endpoints {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Determine if satellite pixels overlap.
    ///
    /// Overlapping is defined as one pixel having a vertex / corner that is interior to the other
    /// one or as pixels having edges that intersect.
    ///
    /// The eps parameter is used as a parameter for any cases where floating point values need to
    /// be compared. There are a few places in the algorithm where that happens, and if they are
    /// within eps units of each other, they are considered equal.
    #[rustfmt::skip]
    pub fn overlap(&self, other: &Pixel, eps: f64) -> bool {
        use crate::geo::Line;

        // Check if they are equal first, then of course they overlap!
        if self.approx_equal(other, eps) {
            return true;
        }

        // Check the bounding boxes, if they don't overlap there is no way these do.
        if !self.bounding_box().overlap(&other.bounding_box(), eps) {
            return false;
        }

        // If pixels overlap, then at least 1 vertex from one pixel must be inside the boundary of
        // the other pixel or the pixels must have lines that intersect. In the case of one pixel
        // completely contained inside another (extremely unlikely), there would be no intersections
        // but all the points of one would be contained in another. In any other case, there must be
        // an intersection of lines.
        //
        // This is all by my own reasoning, not based on any math book or papers on geometry. I'm
        // assuming all pixels are convex quadrilaterals.

        // Check for intersecting lines between the pixels.
        let self_lines = [
            Line {start: self.ul, end: self.ur},
            Line {start: self.ur, end: self.lr},
            Line {start: self.lr, end: self.ll},
            Line {start: self.ll, end: self.ul},
        ];

        let other_lines = [
            Line {start: other.ul, end: other.ur},
            Line {start: other.ur, end: other.lr},
            Line {start: other.lr, end: other.ll},
            Line {start: other.ll, end: other.ul},
        ];

        for s_line in self_lines {
            for o_line in other_lines {
                if let Some(res) = s_line.intersect(o_line, eps) {
                    if !res.intersect_is_endpoints {
                        return true;
                    }
                }
            }
        }

        // Checking for intersecting lines didn't find anything. Now try seeing if one pixel is
        // contained in the other pixel.
        let self_coords = [self.ul, self.ur, self.lr, self.ll];
        for coord in self_coords {
            if other.contains_coord(coord, eps) {
                return true;
            }
        }

        // Why not check the other of other_coords are inside self? Because I think you can
        // convince yourself geometrically that if that is the case, then the last check would also
        // have to be true!
        //
        //let other_coords = [other.ul, other.ur, other.lr, other.ll];
        //for coord in other_coords {
        //    if self.contains_coord(coord, eps) {
        //        return true;
        //    }
        //}

        // No intersecting lines and no corners of one pixel contained in the other, so there
        // is no overlap.
        false
    }

    /// Determine if satellite pixels are adjacent.
    ///
    /// Adjacent is defined as having at least one corner that is `eps` close to a coordinate in the
    /// other. Adjacent pixels may overlap also because the overlap method uses the `eps` variable
    /// in determining overlap. However, if there is a large overlap, the pixels aren't adjacent.
    ///
    /// # Arguments
    ///
    /// * `other` - the pixel to check against.
    /// * `eps` - The scale to use for comparison in the same units as the lat and lon.
    pub fn is_adjacent_to(&self, other: &Pixel, eps: f64) -> bool {
        // If they are the same Pixel, then they overlap too much to be adjacent.
        if self.approx_equal(other, eps) {
            return false;
        }

        // If the bounding boxes don't overlap, this isn't going to workout either.
        if !self.bounding_box().overlap(&other.bounding_box(), eps) {
            return false;
        }

        let self_coords = [self.ul, self.ur, self.lr, self.ll];
        let other_coords = [other.ul, other.ur, other.lr, other.ll];

        // Count the number of close coords and mark which ones are close.
        let mut self_close = [false, false, false, false];
        let mut other_close = [false, false, false, false];
        let mut num_close_coords = 0;
        for i in 0..self_coords.len() {
            for j in 0..other_coords.len() {
                if self_coords[i].is_close(other_coords[j], eps) {
                    num_close_coords += 1;
                    self_close[i] = true;
                    other_close[j] = true;
                }
            }
        }

        // bail out early if we can
        if num_close_coords < 1 || num_close_coords > 2 {
            return false;
        }

        // Check if any not close points are contained in the other pixel
        for i in 0..self_close.len() {
            if !self_close[i] {
                if other.contains_coord(self_coords[i], eps) {
                    return false;
                }
            }

            if !other_close[i] {
                if self.contains_coord(other_coords[i], eps) {
                    return false;
                }
            }
        }

        // The following is a heuristic that should catch most of the remaining edge cases. For the
        // satellite data this program will be working with, this should really be more than good
        // enough.

        // If they are adjacent, the centroid of neither should be interior to the other.
        let self_centroid = self.centroid();
        if other.contains_coord(self_centroid, eps) {
            return false;
        }

        let other_centroid = other.centroid();
        if self.contains_coord(other_centroid, eps) {
            return false;
        }

        true
    }

    /// Determine if satellite pixels are adjacent or overlapping.
    pub fn is_adjacent_to_or_overlaps(&self, other: &Pixel, eps: f64) -> bool {
        // Try some shortcuts first
        if !self.bounding_box().overlap(&other.bounding_box(), eps) {
            return false;
        }

        let self_coords = [self.ul, self.ur, self.lr, self.ll];
        let other_coords = [other.ul, other.ur, other.lr, other.ll];

        // Count the number of close coords
        let mut num_close_coords = 0;
        for s_coord in self_coords {
            for o_coord in other_coords {
                if s_coord.is_close(o_coord, eps) {
                    num_close_coords += 1;

                    // bail out early if we can
                    if num_close_coords > 1 {
                        return true;
                    }
                }
            }
        }

        // Check if any points are contained in the other pixel
        for s_coord in self_coords {
            if other.contains_coord(s_coord, eps) {
                return true;
            }
        }

        for o_coord in other_coords {
            if self.contains_coord(o_coord, eps) {
                return true;
            }
        }

        // Fallback to the tested methods.
        self.overlap(other, eps) || self.is_adjacent_to(other, eps)
    }
}

/// A pixel list stores a list of Pixel objects.
#[derive(Debug, Clone)]
pub struct PixelList(Vec<Pixel>);

impl Geo for PixelList {
    fn centroid(&self) -> Coord {
        let mut centroid = Coord { lat: 0.0, lon: 0.0 };
        for pixel in &self.0 {
            let coord = pixel.centroid();
            centroid.lat += coord.lat;
            centroid.lon += coord.lon;
        }

        centroid.lat /= self.0.len() as f64;
        centroid.lon /= self.0.len() as f64;

        return centroid;
    }

    #[rustfmt::skip]
    fn bounding_box(&self) -> BoundingBox {
        let mut min_lat = std::f64::INFINITY;
        let mut max_lat = -std::f64::INFINITY;
        let mut min_lon = std::f64::INFINITY;
        let mut max_lon = -std::f64::INFINITY;

        for pixel in &self.0 {
            min_lat = min_lat.min(pixel.ll.lat).min(pixel.lr.lat);
            max_lat = max_lat.max(pixel.ul.lat).max(pixel.ur.lat);
            min_lon = min_lon.min(pixel.ll.lon).min(pixel.lr.lon);
            max_lon = max_lon.max(pixel.ul.lon).max(pixel.ur.lon);
        }

        BoundingBox {ll: Coord {lat: min_lat, lon: min_lon}, ur: Coord {lat: max_lat, lon: max_lon}}
    }
}

impl PixelList {
    /// Create a new PixelList
    pub fn new() -> Self {
        PixelList(vec![])
    }

    /// Create a new PixelList with a given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        PixelList(Vec::with_capacity(capacity))
    }

    /// Append a [Pixel] to the end of the list.
    pub fn push(&mut self, pixel: Pixel) {
        self.0.push(pixel)
    }

    /// Empty the list, but keep it intact for reuse.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Calculate the total power in a PixelList, megawatts.
    pub fn total_power(&self) -> f64 {
        self.0
            .iter()
            .filter(|p| !p.power.is_infinite() && !p.power.is_nan())
            .map(|p| p.power)
            .sum()
    }

    /// Calculate the total fire area in a PixelList, square meters.
    pub fn total_are(&self) -> f64 {
        self.0
            .iter()
            .filter(|p| !p.area.is_infinite() && !p.area.is_nan())
            .map(|p| p.area)
            .sum()
    }

    /// Calculate the maximum fire temperature in a PixelList, kelvin.
    pub fn maximum_temperature(&self) -> f64 {
        self.0
            .iter()
            .filter(|p| !p.temperature.is_infinite() && !p.temperature.is_nan())
            .map(|p| p.temperature)
            .fold(-std::f64::INFINITY, |acc, t| acc.max(t))
    }

    /// Calculate the maximum scan angle in a PixelList, degrees.
    pub fn maximum_scan_angle(&self) -> f64 {
        self.0
            .iter()
            .filter(|p| !p.scan_angle.is_infinite() && !p.scan_angle.is_nan())
            .map(|p| p.scan_angle)
            .fold(-std::f64::INFINITY, |acc, t| acc.max(t))
    }

    /// Check to see if these two PixelList objects are adjacent or overlapping.
    pub fn adjacent_to_or_overlaps(&self, other: &PixelList, eps: f64) -> bool {
        if !self.bounding_box().overlap(&other.bounding_box(), eps) {
            return false;
        }

        for s_pixel in &self.0 {
            for o_pixel in &other.0 {
                if s_pixel.is_adjacent_to_or_overlaps(o_pixel, eps) {
                    return true;
                }
            }
        }

        false
    }
}

/*-------------------------------------------------------------------------------------------------
 *                                         Binary Format
 *-----------------------------------------------------------------------------------------------*/
impl PixelList {
    /// Encode the PixelList into a binary format suitable for storing in a database.
    ///
    /// At this time it doesn't support a portable format, meaning no adjustments are made for
    /// endianness or any padding in the array.
    pub fn binary_serialize(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(
            std::mem::size_of::<Pixel>() * self.0.len() + 2 * std::mem::size_of::<usize>(),
        );

        output.write(&self.0.len().to_ne_bytes()).unwrap();
        // Do it again for compatibility with the original type coded in C.
        output.write(&self.0.len().to_ne_bytes()).unwrap();
        for pixel in &self.0 {
            pixel.write_bytes(&mut output).unwrap();
        }

        output
    }

    /// Deserialize an array of bytes into a PixelList.
    ///
    pub fn binary_deserialize<R: Read>(r: &mut R) -> Self {
        let mut buf: [u8; std::mem::size_of::<usize>()] = [0; std::mem::size_of::<usize>()];

        r.read_exact(&mut buf).unwrap();
        let len = usize::from_ne_bytes(buf);

        // Read the "Capacity" variable from the old C binary format.
        r.read_exact(&mut buf).unwrap();

        let mut data: Vec<Pixel> = Vec::with_capacity(len);

        for _ in 0..len {
            data.push(Pixel::read_bytes(r));
        }

        PixelList(data)
    }
}

/*-------------------------------------------------------------------------------------------------
 *                                         KML Export
 *-----------------------------------------------------------------------------------------------*/

impl PixelList {
    fn kml_write_pixel_style(kml: &mut KmlFile, mut power: f64) {
        const MAX_POWER: f64 = 3_000.0;
        const MAX_GREEN_FOR_ORANGE: f64 = 0.647;
        const FULL_RED_POWER: f64 = MAX_POWER / 2.0;

        let rd = 1.0;
        let gd;
        let mut bd = 0.0;
        let ad = 0.6;

        if power.is_infinite() {
            power = MAX_POWER;
        }

        power = power.min(MAX_POWER);

        if power <= FULL_RED_POWER {
            gd = (FULL_RED_POWER - power) / FULL_RED_POWER * MAX_GREEN_FOR_ORANGE;
        } else {
            gd = (power - FULL_RED_POWER) / (MAX_POWER - FULL_RED_POWER);
            bd = gd;
        }

        let ri = (rd * 255.0) as i32;
        let gi = (gd * 255.0) as i32;
        let bi = (bd * 255.0) as i32;
        let ai = (ad * 255.0) as i32;

        debug_assert!(ri < 256 && gi < 256 && bi < 256 && ai < 256);
        debug_assert!(ri >= 0 && gi >= 0 && bi >= 0 && ai >= 0);

        let mut color: [u8; 9] = [0; 9];
        let mut cursor = std::io::Cursor::new(&mut color[..]);
        let _ = write!(cursor, "{:02X}{:02X}{:02X}{:02X}", ai, bi, gi, ri);

        let color = std::str::from_utf8(&color[0..8]).unwrap();

        let _ = kml.start_style(None);
        let _ = kml.create_poly_style(Some(color), true, false);
        let _ = kml.finish_style();
    }

    /// Write out a pixel list in KML format.
    ///
    /// This will print out a multigeometry KML element. It should be composed as part of a function
    /// that outputs a KML file where that higher function adds style information and the rest of the
    /// document.
    ///
    pub(crate) fn kml_write(&self, kml: &mut KmlFile) {
        for pixel in &self.0 {
            let mut desc: [u8; 256] = [0; 256];
            let mut cursor = std::io::Cursor::new(&mut desc[..]);

            let _ = write!(
                cursor,
                concat!(
                    "Power: {:.0} MW<br/>",
                    "Area: {:.0} m^2</br>",
                    "Temperature: {:.0} K<br/>",
                    "scan angle: {:.0}&deg;<br/>",
                    "Mask Flag: {}<br/>",
                    "Data Quality Flag: {}<br/>"
                ),
                pixel.power,
                pixel.area,
                pixel.temperature,
                pixel.scan_angle,
                pixel.mask_flag.as_str(),
                pixel.data_quality_flag.as_str()
            );

            let desc = unsafe { std::str::from_utf8_unchecked(&desc) };
            let _ = kml.start_placemark(None, Some(desc), None);

            Self::kml_write_pixel_style(kml, pixel.power);
            let _ = kml.start_polygon(true, true, Some("clampToGround"));
            let _ = kml.polygon_start_outer_ring();
            let _ = kml.start_linear_ring();

            let _ = kml.linear_ring_add_vertex(pixel.ul.lat, pixel.ul.lon, 0.0);
            let _ = kml.linear_ring_add_vertex(pixel.ll.lat, pixel.ll.lon, 0.0);
            let _ = kml.linear_ring_add_vertex(pixel.lr.lat, pixel.lr.lon, 0.0);
            let _ = kml.linear_ring_add_vertex(pixel.ur.lat, pixel.ur.lon, 0.0);

            // Close the loop.
            let _ = kml.linear_ring_add_vertex(pixel.ul.lat, pixel.ul.lon, 0.0);
            let _ = kml.finish_linear_ring();
            let _ = kml.polygon_finish_outer_ring();
            let _ = kml.finish_polygon();
            let _ = kml.finish_placemark();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn test_satfire_pixel_centroid() {
        let pxl = Pixel { 
            ul: Coord {lat: 45.0, lon: -120.0},
            ll: Coord {lat: 44.0, lon: -120.0},
            lr: Coord {lat: 44.0, lon: -119.0},
            ur: Coord {lat: 45.0, lon: -119.0},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        let centroid = Coord {lat: 44.5, lon: -119.5};
        let centroid_calc = pxl.centroid();

        assert!(centroid.is_close(centroid_calc, 1.0e-12));
    }

    #[test]
    #[rustfmt::skip]
    fn test_satfire_pixels_approx_equal() {
        let pxl1 = Pixel {
            ul: Coord {lat: 45.0, lon: -120.0},
            ll: Coord {lat: 44.0, lon: -120.0},
            lr: Coord {lat: 44.0, lon: -119.0},
            ur: Coord {lat: 45.0, lon: -119.0},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        let pxl2 = Pixel {
            ul: Coord {lat: 45.0000002, lon: -120.0000002},
            ll: Coord {lat: 44.0000002, lon: -119.9999998},
            lr: Coord {lat: 43.9999998, lon: -119.0000002},
            ur: Coord {lat: 44.9999998, lon: -118.9999998},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        assert!(pxl1.approx_equal(&pxl1, 1.0e-6));
        assert!(pxl2.approx_equal(&pxl2, 1.0e-6));
        assert!(pxl1.approx_equal(&pxl2, 1.0e-6));

        assert!(!pxl1.approx_equal(&pxl2, 1.0e-8));
    }

    #[test]
    #[rustfmt::skip]
    fn test_satfire_pixel_contains_coord() {
        // This is a simple square of width & height 1 degree of latitude & longitude
        let pxl1 = Pixel {
            ul: Coord{lat: 45.0, lon: -120.0},
            ll: Coord{lat: 44.0, lon: -120.0},
            lr: Coord{lat: 44.0, lon: -119.0},
            ur: Coord{lat: 45.0, lon: -119.0},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        let inside1 = Coord {lat: 44.5, lon: -119.5};

        let outside1 = Coord {lat: 45.5, lon: -119.5};
        let outside2 = Coord {lat: 44.5, lon: -120.5};
        let outside3 = Coord {lat: 43.5, lon: -119.5};
        let outside4 = Coord {lat: 44.5, lon: -118.5};
        let outside5 = Coord {lat: 43.5, lon: -118.5};
        let outside6 = Coord {lat: 45.5, lon: -120.5};

        let boundary1 = Coord {lat: 45.0, lon: -119.5};
        let boundary2 = Coord {lat: 44.0, lon: -119.5};
        let boundary3 = Coord {lat: 44.5, lon: -119.0};
        let boundary4 = Coord {lat: 44.5, lon: -120.0};

        // Make sure what's inside is in
        assert!(pxl1.contains_coord(inside1, 1.0e-6));

        // Make sure what's outside is out
        assert!(!pxl1.contains_coord(outside1, 1.0e-6));
        assert!(!pxl1.contains_coord(outside2, 1.0e-6));
        assert!(!pxl1.contains_coord(outside3, 1.0e-6));
        assert!(!pxl1.contains_coord(outside4, 1.0e-6));
        assert!(!pxl1.contains_coord(outside5, 1.0e-6));
        assert!(!pxl1.contains_coord(outside6, 1.0e-6));

        // Make sure what lies on the boundary is NOT contained in the polygon.
        assert!(!pxl1.contains_coord(boundary1, 1.0e-6));
        assert!(!pxl1.contains_coord(boundary2, 1.0e-6));
        assert!(!pxl1.contains_coord(boundary3, 1.0e-6));
        assert!(!pxl1.contains_coord(boundary4, 1.0e-6));

        // This is a very skewed quadrilateral
        let pxl2 = Pixel {
            ul: Coord{lat: 3.0, lon: 2.0},
            ll: Coord{lat: 0.0, lon: 0.0},
            lr: Coord{lat: 2.0, lon: 2.0},
            ur: Coord{lat: 5.0, lon: 4.0},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        let inside1 = Coord {lat: 2.5, lon: 2.0};

        let outside1 = Coord {lat: 2.0, lon: 1.0};
        let outside2 = Coord {lat: 1.0, lon: 2.0};
        let outside3 = Coord {lat: -1.5, lon: -119.5};

        let boundary1 = Coord {lat: 1.0, lon: 1.0};
        let boundary2 = Coord {lat: 4.0, lon: 3.0};

        // Make sure what's inside is in
        assert!(pxl2.contains_coord(inside1, 1.0e-6));

        // Make sure what's outside is out
        assert!(!pxl2.contains_coord(outside1, 1.0e-6));
        assert!(!pxl2.contains_coord(outside2, 1.0e-6));
        assert!(!pxl2.contains_coord(outside3, 1.0e-6));

        // Make sure what lies on the boundary is NOT contained in the polygon.
        assert!(!pxl2.contains_coord(boundary1, 1.0e-6));
        assert!(!pxl2.contains_coord(boundary2, 1.0e-6));
    }

    #[test]
    #[rustfmt::skip]
    fn test_satfire_pixels_overlap() {
        let pxl1 = Pixel {
            ul: Coord{lat: 45.0, lon: -120.0},
            ll: Coord{lat: 44.0, lon: -120.0},
            lr: Coord{lat: 44.0, lon: -119.0},
            ur: Coord{lat: 45.0, lon: -119.0},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        let pxl2 = Pixel {
            ul: Coord{lat: 45.5, lon: -120.5},
            ll: Coord{lat: 44.5, lon: -120.5},
            lr: Coord{lat: 44.5, lon: -119.5},
            ur: Coord{lat: 45.5, lon: -119.5},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        let pxl3 = Pixel {
            ul: Coord{lat: 46.0, lon: -120.0},
            ll: Coord{lat: 45.0, lon: -120.0},
            lr: Coord{lat: 45.0, lon: -119.0},
            ur: Coord{lat: 46.0, lon: -119.0},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        // The corners of pxl4 lie along the mid-points of pxl1. So they overlap.
        let pxl4 = Pixel {
            ul: Coord{lat: 45.0, lon: -119.5},
            ll: Coord{lat: 44.5, lon: -120.0},
            lr: Coord{lat: 44.0, lon: -119.5},
            ur: Coord{lat: 44.5, lon: -119.0},
            power: 0.0,
            area: 0.0,
            temperature: 0.0,
            scan_angle: 0.0,
            mask_flag: MaskCode(0),
            data_quality_flag: DataQualityFlagCode(0),
        };

        // pixels are always overlapping themselves.
        assert!(pxl1.overlap(&pxl1, 1.0e-6));
        assert!(pxl2.overlap(&pxl2, 1.0e-6));
        assert!(pxl3.overlap(&pxl3, 1.0e-6));
        assert!(pxl4.overlap(&pxl4, 1.0e-6));

        assert!(pxl1.is_adjacent_to_or_overlaps(&pxl1, 1.0e-6));
        assert!(pxl2.is_adjacent_to_or_overlaps(&pxl2, 1.0e-6));
        assert!(pxl3.is_adjacent_to_or_overlaps(&pxl3, 1.0e-6));
        assert!(pxl4.is_adjacent_to_or_overlaps(&pxl4, 1.0e-6));

        // pxl1 and pxl3 are adjacent, but they do not overlap. However, the corners are close
        // enough that with the `eps`, they do overlap.
        assert!(pxl1.overlap(&pxl3, 1.0e-6));
        assert!(pxl3.overlap(&pxl1, 1.0e-6));

        assert!(pxl1.is_adjacent_to_or_overlaps(&pxl3, 1.0e-6));
        assert!(pxl3.is_adjacent_to_or_overlaps(&pxl1, 1.0e-6));

        // pxl2 overlaps pxl1 and pxl3 - order doesn't matter
        assert!(pxl1.overlap(&pxl2, 1.0e-6));
        assert!(pxl2.overlap(&pxl1, 1.0e-6));

        assert!(pxl1.is_adjacent_to_or_overlaps(&pxl2, 1.0e-6));
        assert!(pxl2.is_adjacent_to_or_overlaps(&pxl1, 1.0e-6));

        assert!(pxl3.overlap(&pxl2, 1.0e-6));
        assert!(pxl2.overlap(&pxl3, 1.0e-6));

        assert!(pxl3.is_adjacent_to_or_overlaps(&pxl2, 1.0e-6));
        assert!(pxl2.is_adjacent_to_or_overlaps(&pxl3, 1.0e-6));

        // Test the case where a vertex lies on the boundary.
        assert!(pxl1.overlap(&pxl4, 1.0e-6));
        assert!(pxl4.overlap(&pxl1, 1.0e-6));

        assert!(pxl1.is_adjacent_to_or_overlaps(&pxl4, 1.0e-6));
        assert!(pxl4.is_adjacent_to_or_overlaps(&pxl1, 1.0e-6));
    }


}
