#pragma once
/**
 * \file satellite.h
 *
 * \brief Metadata about the satellite platforms.
 */

#include <time.h>

#include "geo.h"

/** \brief The GOES satellites this library works with. */
enum Satellite {
    SATFIRE_SATELLITE_G16, /**< GOES-16, formerly GOES-R. */
    SATFIRE_SATELLITE_G17, /**< GOES-17, formerly GOES-S. */
    SATFIRE_SATELLITE_NUM, /**< The number of satellites in the enumeration. */
    SATFIRE_SATELLITE_NONE = SATFIRE_SATELLITE_NUM /**< Used as an error code. */
};

/** \brief The satellite scan sectors this library recognizes. */
enum Sector {
    SATFIRE_SECTOR_FULL,  /**< The full disk scan sector. */
    SATFIRE_SECTOR_CONUS, /**< The CONUS, Continental U.S. scan sector. */
    SATFIRE_SECTOR_MESO1, /**< There are two floating meso sectors. */
    SATFIRE_SECTOR_MESO2, /**< There are two floating meso sectors. */
    SATFIRE_SECTOR_NUM,   /**< The number of valid members in this enumeration. */
    SATFIRE_SECTOR_NONE = SATFIRE_SECTOR_NUM /**< Used as an error code. */
};

/** \brief Get a string representing the name of the satellite. */
char const *satfire_satellite_name(enum Satellite const sat);

/** \brief Scan the string for the occurrence of a satellite name and return the first one found.
 *
 * \returns Satellite that corresponds to the first satellite name found, or SATFIRE_SATELLITE_NONE
 * if none was found.
 */
enum Satellite satfire_satellite_string_contains_satellite(char const *str);

/** \brief Get the earliest operational date for the satellite. */
time_t satfire_satellite_operational(enum Satellite const sat);

/** \brief Get the area we consider as valid for fire data characterization.
 *
 * This is set up so we can easily exclude data near the limb of the Earth as viewed by the
 * satellite. Early investigations into the data have shown a lot of suspicous data in these areas.
 */
struct BoundingBox satfire_satellite_data_area(enum Satellite const sat);

/** \brief Get a string representing the sector. */
char const *satfire_sector_name(enum Sector const sector);

/** \brief Scan the string for the occurrence of a sector name and return the first one found.
 *
 * \returns Sector that corresponds to the first sector name found, or SATFIRE_SECTOR_NONE
 * if none was found.
 */
enum Sector satfire_sector_string_contains_sector(char const *str);
