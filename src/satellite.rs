/*! Contains all the information about satellites. */

use chrono::{NaiveDate, NaiveDateTime};

/** The GOES satellites this library works with. */
#[derive(Debug, Clone, Copy)]
pub enum Satellite {
    /// GOES-16 (formerly GOES-R), or commonly known as GOES East
    G16,
    /// GOES-17 (formerly GOES-S), or commonly known as GOES West
    G17,
}

impl Satellite {
    /// Get a string representing the name of the satellite.
    pub fn name(&self) -> &'static str {
        use Satellite::*;

        match self {
            G16 => "G16",
            G17 => "G17",
        }
    }

    /// Scan the string for the occurence of a satellite name.
    pub fn string_contains_satellite(string: &str) -> Option<Satellite> {
        use Satellite::*;

        let all_sats = [G16, G17];
        for sat in &all_sats {
            if string.contains(sat.name()) {
                return Some(*sat);
            }
        }

        None
    }

    /// Get the date and time (in UTC) that the satellite became operational.
    ///
    /// This is the time that the satellite was officially declared operational after all checkouts
    /// and operational testing. It may have started sending data before this date, but it may not
    /// be trustworthy data.
    pub fn operational(&self) -> NaiveDateTime {
        use Satellite::*;

        match self {
            G16 => NaiveDate::from_ymd(2017, 12, 18).and_hms(12, 0, 0),
            G17 => NaiveDate::from_ymd(2019, 2, 12).and_hms(12, 0, 0),
        }
    }
}

/** The satellite scan sectors this library recognizes. */
#[derive(Debug, Clone, Copy)]
pub enum Sector {
    /// This is the full disk sector that includes the full viewable disk of the Earth.
    FULL,
    /// The Continental U.S. sector, which actually includes much of the U.S on both satellites.
    CONUS,
    /// Meso-sector 1, a floating sector with 1 minute imagery updates.
    MESO1,
    /// Meso-sector 2, a floating sector with 1 minute imagery updates.
    MESO2,
}

impl Sector {
    /// Get a string representing the name of the sector.
    ///
    /// This is also the abbreviation used for the sector in the NOAA Big Data file naming scheme
    /// for GOES data.
    pub fn name(&self) -> &'static str {
        use Sector::*;

        match self {
            FULL => "FDCF",
            CONUS => "FDCC",
            MESO1 => "FDCM1",
            MESO2 => "FDCM2",
        }
    }

    /// Scan the string for the occurrence of a sector name and return first one found.
    ///
    /// Note that in some cases either of the meso-sectors can be represented by "FDCM", such as in
    /// the directory structure where both meso-sector files are stored in the same directory. So
    /// "FDCM" is the last string that the function will try to match and it will just return
    /// Sector::FDCM1 in that case.
    pub fn string_contains_sector(string: &str) -> Option<Sector> {
        use Sector::*;

        let all_sectors = [FULL, CONUS, MESO1, MESO2];
        for sector in all_sectors {
            if string.contains(sector.name()) {
                return Some(sector);
            }
        }

        if string.contains("FDCM") {
            Some(MESO1)
        } else {
            None
        }
    }
}

/// Represents a code from the Mask field of the NetCDF files.
#[derive(Debug, Clone, Copy)]
pub struct MaskCode(pub i16);

impl MaskCode {
    /// Translate a mask code to a string.
    ///
    /// Mask codes are a form of metadata that describe each pixel's quality control characteristics.
    /// These codes were taken from table 5.19.6.1-1 of the
    /// [GOES-R SERIES PRODUCT DEFINITION AND USERS’ GUIDE][doc_url] retrieved December 10th, 2021.
    ///
    /// [doc_url]: https://www.goes-r.gov/products/docs/PUG-L2+-vol5.pdf
    pub fn as_str(self) -> &'static str {
        match self.0 {
            -99 => "missing",
            0 => "unprocessed_pixel",
            10 => "good_fire_pixel",
            11 => "saturated_fire_pixel",
            12 => "cloud_contaminated_fire_pixel",
            13 => "high_probability_fire_pixel",
            14 => "medium_probability_fire_pixel",
            15 => "low_probability_fire_pixel",
            30 => "temporally_filtered_good_fire_pixel",
            31 => "temporally_filtered_saturated_fire_pixel",
            32 => "temporally_filtered_cloud_contaminated_fire_pixel",
            33 => "temporally_filtered_high_probability_fire_pixel",
            34 => "temporally_filtered_medium_probability_fire_pixel",
            35 => "temporally_filtered_low_probability_fire_pixel",
            40 => "off_earth_pixel",
            50 => "LZA_block_out_zone",
            60 => "SZA_or_glint_angle_block_out_zone",
            100 => "processed_no_fire_pixel",
            120 => "missing_input_3.89um_pixel",
            121 => "missing_input_11.19um_pixel",
            123 => "saturated_input_3.89um_pixel",
            124 => "saturated_input_11.19um_pixel",
            125 => "invalid_input_radiance_value",
            126 => "below_threshold_input_3.89um_pixel",
            127 => "below_threshold_input_11.19um_pixel",
            150 => "invalid_ecosystem_UMD_land_cover_type_sea_water_or_MODIS_land_mask_types_or_framework_desert_mask_type_bright_desert",
            151 => "invalid_ecosystem_USGS_type_sea_water",
            152 => "invalid_ecosystem_USGS_types_coastline_fringe_or_compound_coastlines",
            153 => "invalid_ecosystem_USGS_types_inland_water_or_water_and_island_fringe_or_land_and_water_shore_or_land_and_water_rivers",
            170 => "no_background_value_could_be_computed",
            180 => "conversion_error_between_BT_and_radiance",
            182 => "conversion_error_radiance_to_adjusted_BT",
            185 => "modified_Dozier_technique_bisection_method_invalid_computed_BT",
            186 => "modifed_Dozier_technique_Newton_method_invalid_computed_radiance",
            187 => "modifed_Dozier_technique_Newton_method_invalid_computed_fire_brighness_temp",
            188 => "modifed_Dozier_technique_Newton_method_invalid_computed_fire_area",
            200 => "cloud_pixel_detected_by_11.19um_threshold_test",
            201 => "cloud_pixel_detected_by_3.89um_minus_11.19um_threshold_and_freezing_test",
            205 => "cloud_pixel_detected_by_negative_difference_3.89um_minus_11.19um_threshold_test",
            210 => "cloud_pixel_detected_by_positive_difference_3.89um_minus_11.19um_threshold_test",
            215 => "cloud_pixel_detected_by_albedo_threshold_test",
            220 => "cloud_pixel_detected_by_12.27um_threshold_test",
            225 => "cloud_pixel_detected_by_negative_difference_11.19um_minus_12.27um_threshold_test",
            230 => "cloud_pixel_detected_by_positive_difference_11.19um_minus_12.27um_threshold_test",
            240 => "cloud_edge_pixel_detected_by_along_scan_reflectivity_and_3.89um_threshold_test",
            245 => "cloud_edge_pixel_detected_by_along_scan_reflectivity_and_albedo_threshold_test",
            _ => "unknown code",
        }
    }
}

/// Represents a code from the DQF (Data Quality Flag) field of the NetCDF file.
///
/// DQF codes are a simplified version of the mask codes described above that only tell the result
/// of the quality control analysis.  These codes were taken from table 5.19.6.1-2 of the
/// [GOES-R SERIES PRODUCT DEFINITION AND USERS’ GUIDE][doc_url] retrieved December 10th, 2021.
///
/// [doc_url]: (https://www.goes-r.gov/products/docs/PUG-L2+-vol5.pdf)
#[derive(Debug, Clone, Copy)]
pub struct DataQualityFlagCode(pub i16);

impl DataQualityFlagCode {
    /// Translate a DQF code to a string.
    pub fn as_str(self) -> &'static str {
        match self.0 {
            0 => "good_quality_fire_pixel_qf ",
            1 => "good_quality_fire_free_land_pixel_qf ",
            2 => "invalid_due_to_opaque_cloud_pixel_qf ",
            3 => "invalid_due_to_surface_type_or_sunglint_or_LZA_threshold_exceeded_or_off_earth_or_missing_input_data_qf ",
            4 => "invalid_due_to_bad_input_data_qf ",
            5 => "invalid_due_to_algorithm_failure_qf",
            255 | -1 => "missing",
            _ => "unknown",
        }
    }
}
