#include "firepoint.h"
#include "geo.h"

#include <math.h>

double
firepoint_great_circle_distance(struct FirePoint const a, struct FirePoint const b)
{
    double a_lat = a.lat * DEG2RAD;
    double b_lat = b.lat * DEG2RAD;
    double a_lon = a.lon * DEG2RAD;
    double b_lon = b.lon * DEG2RAD;

    return great_circle_distance(a_lat, a_lon, b_lat, b_lon);
}
