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

/**
 * \brief the simple great circle distance calculation.
 *
 * \param lat1 the latitude of the first point in degrees.
 * \param lon1 the longitude of the first point in degrees.
 * \param lat2 the latitude of the second point in degrees.
 * \param lon2 the longitude of the second point in degrees.
 *
 * \return the distance between the points in kilometers.
 */
double great_circle_distance(double lat1, double lon1, double lat2, double lon2);
