#include "satellite.h"

#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

char const *
satfire_satellite_name(enum Satellite const sat)
{
    assert(sat == SATFIRE_SATELLITE_G16 || sat == SATFIRE_SATELLITE_G17);

    switch (sat) {
    case SATFIRE_SATELLITE_G16:
        return "G16";
    case SATFIRE_SATELLITE_G17:
        return "G17";
    default:
        exit(EXIT_FAILURE);
    }
}

enum Satellite
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
satfire_satellite_operational(enum Satellite const sat)
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

struct BoundingBox
satfire_satellite_data_area(enum Satellite const sat)
{
    // Centered over -75.2
    // -26.0 Longitude to get Africa and the islands off it's west coast out, they are a common
    // source of false detections.
    static struct BoundingBox const G16_BB = {.ll = (struct Coord){.lat = -60.0, .lon = -135.0},
                                              .ur = (struct Coord){.lat = 60.0, .lon = -26.0}};

    // Centered over -137.2
    static struct BoundingBox const G17_BB = {.ll = (struct Coord){.lat = -60.0, .lon = -180.0},
                                              .ur = (struct Coord){.lat = 60.0, .lon = -77.0}};

    assert(sat == SATFIRE_SATELLITE_G16 || sat == SATFIRE_SATELLITE_G17);

    switch (sat) {
    case SATFIRE_SATELLITE_G16:
        return G16_BB;
    case SATFIRE_SATELLITE_G17:
        return G17_BB;
    default:
        exit(EXIT_FAILURE);
    }
}
