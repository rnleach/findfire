#include "satfire.h"

#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

char const *
satfire_satellite_name(enum SFSatellite const sat)
{
    assert(sat == SATFIRE_SATELLITE_G16 || sat == SATFIRE_SATELLITE_G17 ||
           sat == SATFIRE_SATELLITE_NONE);

    switch (sat) {
    case SATFIRE_SATELLITE_G16:
        return "G16";
    case SATFIRE_SATELLITE_G17:
        return "G17";
    case SATFIRE_SATELLITE_NONE:
        return "NONE";
    default:
        exit(EXIT_FAILURE);
    }
}

enum SFSatellite
satfire_satellite_string_contains_satellite(char const *str)
{
    assert(str);

    if (strstr(str, satfire_satellite_name(SATFIRE_SATELLITE_G16))) {
        return SATFIRE_SATELLITE_G16;
    }

    if (strstr(str, satfire_satellite_name(SATFIRE_SATELLITE_G17))) {
        return SATFIRE_SATELLITE_G17;
    }

    return SATFIRE_SATELLITE_NONE;
}

time_t
satfire_satellite_operational(enum SFSatellite const sat)
{
    assert(sat == SATFIRE_SATELLITE_G16 || sat == SATFIRE_SATELLITE_G17);

    static struct tm G16_Oper = {.tm_sec = 0,
                                 .tm_min = 0,
                                 .tm_hour = 12,
                                 .tm_mday = 18,
                                 .tm_mon = 11,
                                 .tm_year = 2017 - 1900};

    static struct tm G17_Oper = {.tm_sec = 0,
                                 .tm_min = 0,
                                 .tm_hour = 12,
                                 .tm_mday = 12,
                                 .tm_mon = 1,
                                 .tm_year = 2019 - 1900};
    struct tm *target = 0;
    switch (sat) {
    case SATFIRE_SATELLITE_G16:
        target = &G16_Oper;
        break;
    case SATFIRE_SATELLITE_G17:
        target = &G17_Oper;
        break;
    default:
        exit(EXIT_FAILURE);
    }

    return timegm(target);
}

char const *
satfire_sector_name(enum SFSector const sector)
{
    assert(sector == SATFIRE_SECTOR_FULL || sector == SATFIRE_SECTOR_CONUS ||
           sector == SATFIRE_SECTOR_MESO1 || sector == SATFIRE_SECTOR_MESO2 ||
           sector == SATFIRE_SECTOR_NONE);

    switch (sector) {
    case SATFIRE_SECTOR_FULL:
        return "FDCF";
    case SATFIRE_SECTOR_CONUS:
        return "FDCC";
    case SATFIRE_SECTOR_MESO1:
        return "FDCM1";
    case SATFIRE_SECTOR_MESO2:
        return "FDCM2";
    case SATFIRE_SECTOR_NONE:
        return "NONE";
    default:
        exit(EXIT_FAILURE);
    }
}

enum SFSector
satfire_sector_string_contains_sector(char const *str)
{
    assert(str);

    if (strstr(str, satfire_sector_name(SATFIRE_SECTOR_FULL))) {
        return SATFIRE_SECTOR_FULL;
    }

    if (strstr(str, satfire_sector_name(SATFIRE_SECTOR_CONUS))) {
        return SATFIRE_SECTOR_CONUS;
    }

    if (strstr(str, satfire_sector_name(SATFIRE_SECTOR_MESO1))) {
        return SATFIRE_SECTOR_MESO1;
    }

    if (strstr(str, satfire_sector_name(SATFIRE_SECTOR_MESO2))) {
        return SATFIRE_SECTOR_MESO2;
    }

    return SATFIRE_SECTOR_NONE;
}

char const *
satfire_satellite_mask_code_to_string(short code)
{
    switch (code) {
    case -99:
        return "missing";
    case 0:
        return "unprocessed_pixel";
    case 10:
        return "good_fire_pixel";
    case 11:
        return "saturated_fire_pixel";
    case 12:
        return "cloud_contaminated_fire_pixel";
    case 13:
        return "high_probability_fire_pixel";
    case 14:
        return "medium_probability_fire_pixel";
    case 15:
        return "low_probability_fire_pixel";
    case 30:
        return "temporally_filtered_good_fire_pixel";
    case 31:
        return "temporally_filtered_saturated_fire_pixel";
    case 32:
        return "temporally_filtered_cloud_contaminated_fire_pixel";
    case 33:
        return "temporally_filtered_high_probability_fire_pixel";
    case 34:
        return "temporally_filtered_medium_probability_fire_pixel";
    case 35:
        return "temporally_filtered_low_probability_fire_pixel";
    case 40:
        return "off_earth_pixel";
    case 50:
        return "LZA_block_out_zone";
    case 60:
        return "SZA_or_glint_angle_block_out_zone";
    case 100:
        return "processed_no_fire_pixel";
    case 120:
        return "missing_input_3.89um_pixel";
    case 121:
        return "missing_input_11.19um_pixel";
    case 123:
        return "saturated_input_3.89um_pixel";
    case 124:
        return "saturated_input_11.19um_pixel";
    case 125:
        return "invalid_input_radiance_value";
    case 126:
        return "below_threshold_input_3.89um_pixel";
    case 127:
        return "below_threshold_input_11.19um_pixel";
    case 150:
        return "invalid_ecosystem_UMD_land_cover_type_sea_water_or_MODIS_land_mask_types_or_"
               "framework_desert_mask_type_bright_desert";
    case 151:
        return "invalid_ecosystem_USGS_type_sea_water";
    case 152:
        return "invalid_ecosystem_USGS_types_coastline_fringe_or_compound_coastlines";
    case 153:
        return "invalid_ecosystem_USGS_types_inland_water_or_water_and_island_fringe_or_land_and_"
               "water_shore_or_land_and_water_rivers";
    case 170:
        return "no_background_value_could_be_computed";
    case 180:
        return "conversion_error_between_BT_and_radiance";
    case 182:
        return "conversion_error_radiance_to_adjusted_BT";
    case 185:
        return "modified_Dozier_technique_bisection_method_invalid_computed_BT";
    case 186:
        return "modifed_Dozier_technique_Newton_method_invalid_computed_radiance";
    case 187:
        return "modifed_Dozier_technique_Newton_method_invalid_computed_fire_brighness_temp";
    case 188:
        return "modifed_Dozier_technique_Newton_method_invalid_computed_fire_area";
    case 200:
        return "cloud_pixel_detected_by_11.19um_threshold_test";
    case 201:
        return "cloud_pixel_detected_by_3.89um_minus_11.19um_threshold_and_freezing_test";
    case 205:
        return "cloud_pixel_detected_by_negative_difference_3.89um_minus_11.19um_threshold_test";
    case 210:
        return "cloud_pixel_detected_by_positive_difference_3.89um_minus_11.19um_threshold_test";
    case 215:
        return "cloud_pixel_detected_by_albedo_threshold_test";
    case 220:
        return "cloud_pixel_detected_by_12.27um_threshold_test";
    case 225:
        return "cloud_pixel_detected_by_negative_difference_11.19um_minus_12.27um_threshold_test";
    case 230:
        return "cloud_pixel_detected_by_positive_difference_11.19um_minus_12.27um_threshold_test";
    case 240:
        return "cloud_edge_pixel_detected_by_along_scan_reflectivity_and_3.89um_threshold_test";
    case 245:
        return "cloud_edge_pixel_detected_by_along_scan_reflectivity_and_albedo_threshold_test";
    default:
        return "unknown code";
    }

    return 0;
}

char const *
satfire_satellite_dqf_code_to_string(unsigned char code)
{
    switch (code) {
    case 0:
        return "good_quality_fire_pixel_qf ";
    case 1:
        return "good_quality_fire_free_land_pixel_qf ";
    case 2:
        return "invalid_due_to_opaque_cloud_pixel_qf ";
    case 3:
        return "invalid_due_to_surface_type_or_sunglint_or_LZA_threshold_exceeded_or_off_earth_or_"
               "missing_input_data_qf ";
    case 4:
        return "invalid_due_to_bad_input_data_qf ";
    case 5:
        return "invalid_due_to_algorithm_failure_qf";
    default:
        return "unknown";
    }
}
