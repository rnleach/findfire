#include "geo.h"

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <tgmath.h>

/*-------------------------------------------------------------------------------------------------
 *                                    Helper types and functions
 *-----------------------------------------------------------------------------------------------*/
struct Line {
    struct Coord start;
    struct Coord end;
};

static struct Coord
lines_intersection(struct Line l1, struct Line l2)
{
    double m1 = (l1.end.lat - l1.start.lat) / (l1.end.lon - l1.start.lon);
    double m2 = (l2.end.lat - l2.start.lat) / (l2.end.lon - l2.start.lon);

    // If either is infinite, this is a vertical line and we need to improve the code to handle
    // that. I don't think that will ever happen. But it could and we should be ready to detect it
    // when it does!
    assert(!isinf(m1));
    assert(!isinf(m2));

    // Parallel lines?! That should NEVER happen with out dataset.
    assert(m1 != m2);

    double x1 = l1.start.lon;
    double y1 = l1.start.lat;
    double x2 = l2.start.lon;
    double y2 = l2.start.lat;

    double x0 = (y2 - y1 + m1 * x1 - m2 * x2) / (m1 - m2);
    double y0 = m1 * (x0 - x1) + y1;

    // Assume that the intersection lies in the range of the original lines for our usecase.
    assert(y0 <= fmax(l1.start.lat, l1.end.lat));
    assert(y0 <= fmax(l2.start.lat, l2.end.lat));
    assert(y0 >= fmin(l1.start.lat, l1.end.lat));
    assert(y0 >= fmin(l2.start.lat, l2.end.lat));

    assert(x0 <= fmax(l1.start.lon, l1.end.lon));
    assert(x0 <= fmax(l2.start.lon, l2.end.lon));
    assert(x0 >= fmin(l1.start.lon, l1.end.lon));
    assert(x0 >= fmin(l2.start.lon, l2.end.lon));

    return (struct Coord){.lat = y0, .lon = x0};
}

static struct Coord
triangle_centroid(struct Coord v1, struct Coord v2, struct Coord v3)
{
    double avg_lat = (v1.lat + v2.lat + v3.lat) / 3.0;
    double avg_lon = (v1.lon + v2.lon + v3.lon) / 3.0;

    return (struct Coord){.lat = avg_lat, .lon = avg_lon};
}

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
    /* Steps to calculatule the centroid of a quadrilateral.
     *
     *  1) Break the quadrilateral into two triangles by creating a diagonal.
     *  2) Calculate the centroid of each triangle by taking the average of it's 3 Coords
     *  3) Create a line connecting the centroids of each triangle.
     *  4) Repeat the process by creating the other diagonal.
     *  5) Find the intersection of the two resulting lines, that is the centroid of the
     *     quadrilateral.
     */

    struct Coord t1_c = triangle_centroid(pxl->ul, pxl->ll, pxl->lr);
    struct Coord t2_c = triangle_centroid(pxl->ul, pxl->ur, pxl->lr);
    struct Line diag1_centroids = {.start = t1_c, .end = t2_c};

    struct Coord t3_c = triangle_centroid(pxl->ul, pxl->ll, pxl->ur);
    struct Coord t4_c = triangle_centroid(pxl->lr, pxl->ur, pxl->ll);
    struct Line diag2_centroids = {.start = t3_c, .end = t4_c};

    return lines_intersection(diag1_centroids, diag2_centroids);
}

bool
sat_pixels_approx_equal(struct SatPixel left[static 1], struct SatPixel right[static 1], double eps)
{
    return coord_are_close(left->ul, right->ul, eps) 
        && coord_are_close(left->ur, right->ur, eps)
        && coord_are_close(left->lr, right->lr, eps)
        && coord_are_close(left->ll, right->ll, eps);
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
