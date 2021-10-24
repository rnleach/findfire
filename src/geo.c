#include "geo.h"

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <tgmath.h>

/*-------------------------------------------------------------------------------------------------
 *                                         Coordinates
 *-----------------------------------------------------------------------------------------------*/
bool
coord_are_close(struct Coord left, struct Coord right, double eps)
{
    double lat_diff = left.lat - right.lat;
    double lon_diff = left.lon - right.lon;
    double distance_squared = lat_diff * lat_diff + lon_diff * lon_diff;

    return distance_squared <= (eps * eps);
}

/*-------------------------------------------------------------------------------------------------
 *                                         SatPixels
 *-----------------------------------------------------------------------------------------------*/
struct Coord
sat_pixel_centroid(struct SatPixel pxl[static 1])
{
    assert(false);
}

bool
sat_pixels_approx_equal(struct SatPixel left[static 1], struct SatPixel right[static 1], double eps)
{
    assert(false);
}

bool
sat_pixels_are_adjacent(struct SatPixel left[static 1], struct SatPixel right[static 1], double eps)
{
    assert(false);
}

bool
sat_pixel_contains_coord(struct SatPixel pxl[static 1], struct Coord coord[static 1])
{
    assert(false);
}

bool
sat_pixels_overlap(struct SatPixel left[static 1], struct SatPixel right[static 1])
{
    assert(false);
}

/*-------------------------------------------------------------------------------------------------
 *                                         PixelList
 *-----------------------------------------------------------------------------------------------*/
struct PixelList *
pixel_list_new()
{
    assert(false);
}

struct PixelList *
pixel_list_new_with_capacity(size_t capacity)
{
    assert(false);
}

void
pixel_list_destroy(struct PixelList *plist[static 1])
{
    assert(false);
}

struct PixelList *
pixel_list_append(struct PixelList list[static 1], struct SatPixel apix[static 1])
{
    assert(false);
}

void
pixel_list_clear(struct PixelList list[static 1])
{
    assert(false);
}

/*-------------------------------------------------------------------------------------------------
 *                                         Binary Format
 *-----------------------------------------------------------------------------------------------*/
size_t
pixel_list_binary_serialize_buffer_size(struct PixelList plist[static 1])
{
    assert(false);
}

size_t
pixel_list_binary_serialize(struct PixelList plist[static 1], size_t buf_size,
                            unsigned char buffer[buf_size])
{
    assert(false);
}

struct PixelList *
pixel_list_binary_deserialize(size_t buf_size, unsigned char buffer[buf_size])
{
    assert(false);
}

/*-------------------------------------------------------------------------------------------------
 *                                         KML Export
 *-----------------------------------------------------------------------------------------------*/
int
pixel_list_kml_print(FILE *strm, struct PixelList plist[static 1])
{
    assert(false);
}

/*-------------------------------------------------------------------------------------------------
 *                                            Misc
 *-----------------------------------------------------------------------------------------------*/
/** Scaling factor to convert from degrees to radians. */
#define DEG2RAD (2.0L * M_PI / 360.0L)
#define EARTH_RADIUS_KM 6371.0090

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

#undef EARTH_RADIUS_KM
#undef DEG2RAD
