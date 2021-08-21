/*!
 * Geographic calculations.
 *
 * Most of this will be done via the GDAL library, but there are some simple (approximate)
 * calculations that aren't in GDAL (probably for accuracy reasons) that I'll implement
 * here.
 */

/**
 * the simple great circle distance calculation.
 *
 * #Arguments
 * * lat1 - the latitude of the first point in degrees.
 * * lon1 - the longitude of the first point in degrees.
 * * lat2 - the latitude of the second point in degrees.
 * * lon2 - the longitude of the second point in degrees.
 *
 * #Returns
 * The distance between the points in kilometers.
 */
pub fn great_circle_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const DEG2RAD: f64 = 2.0 * std::f64::consts::PI / 360.0;
    const EARTH_RADIUS_KM: f64 = 6371.0090;

    let lat1_r = lat1 * DEG2RAD;
    let lon1_r = lon1 * DEG2RAD;
    let lat2_r = lat2 * DEG2RAD;
    let lon2_r = lon2 * DEG2RAD;

    let dlat2 = (lat2_r - lat1_r) / 2.0;
    let dlon2 = (lon2_r - lon1_r) / 2.0;

    let sin2_dlat = f64::powf(f64::sin(dlat2), 2.0);
    let sin2_dlon = f64::powf(f64::sin(dlon2), 2.0);

    let arc = 2.0
        * f64::asin(f64::sqrt(
            sin2_dlat + sin2_dlon * f64::cos(lat1_r) * f64::cos(lat2_r),
        ));

    arc * EARTH_RADIUS_KM
}
