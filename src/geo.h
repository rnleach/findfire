#pragma once
/**
 * \file geo.h
 * \brief Geographic calculations.
 *
 * Most of this will be done via the GDAL library, but there are some simple (approximate)
 * calculations that aren't in GDAL (probably for accuracy reasons) that I'll implement
 * here.
 */

#include <math.h>

/** Scaling factor to convert from degrees to radians. */
#define DEG2RAD (M_PI / 360.0 * 2.0)

/**
 * \brief the simple great circle distance calculation.
 *
 * \param lat1 the latitude of the first point in radians.
 * \param lon1 the longitude of the first point in radians.
 * \param lat2 the latitude of the second point in radians.
 * \param lon2 the longitude of the second point in radians.
 *
 * \return the distance between the points in meters.
 */
double great_circle_distance(double lat1, double lon1, double lat2, double lon2);
