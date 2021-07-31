#include "geo.h"

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
double
great_circle_distance_inner(double lat1, double lon1, double lat2, double lon2)
{
    double dlat2 = (lat2 - lat1) / 2.0;
    double dlon2 = (lon2 - lon1) / 2.0;

    double sin2_dlat = pow(sin(dlat2), 2.0);
    double sin2_dlon = pow(sin(dlon2), 2.0);

    double arc = 2.0 * asin(sqrt(sin2_dlat + sin2_dlon * cos(lat1) * cos(lat2)));

    return arc * 6371009.0;
}
