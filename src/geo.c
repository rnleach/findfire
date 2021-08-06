#include "geo.h"

/** Scaling factor to convert from degrees to radians. */
#define DEG2RAD (2.0 * M_PI / 360.0)

#define EARTH_RADIUS_KM 6371.0090

#include <stdio.h>

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
double
great_circle_distance(double lat1, double lon1, double lat2, double lon2)
{
    double lat1_r = lat1 * DEG2RAD;
    double lon1_r = lon1 * DEG2RAD;
    double lat2_r = lat2 * DEG2RAD;
    double lon2_r = lon2 * DEG2RAD;

    double dlat2 = (lat2_r - lat1_r) / 2.0;
    double dlon2 = (lon2_r - lon1_r) / 2.0;

    double sin2_dlat = pow(sin(dlat2), 2.0);
    double sin2_dlon = pow(sin(dlon2), 2.0);

    double arc = 2.0 * asin(sqrt(sin2_dlat + sin2_dlon * cos(lat1_r) * cos(lat2_r)));

    return arc * EARTH_RADIUS_KM;
}
