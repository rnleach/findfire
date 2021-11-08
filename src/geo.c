#include "geo.h"
#include "util.h"

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <tgmath.h>

/*-------------------------------------------------------------------------------------------------
 *                                    Helper types and functions
 *-----------------------------------------------------------------------------------------------*/
struct Line {
    struct Coord start;
    struct Coord end;
};

struct IntersectResult {
    struct Coord intersection;
    char const *msg;
    bool does_intersect;
    bool intersect_is_endpoints;
};

static bool
line_coord_is_close(struct Line const line, struct Coord const coord, double eps)
{
    struct Coord p0 = coord;
    struct Coord p1 = line.start;
    struct Coord p2 = line.end;
    double eps2 = eps * eps;

    double num = (p2.lon - p1.lon) * (p1.lat - p0.lat) - (p1.lon - p0.lon) * (p2.lat - p1.lat);
    double denom2 = (p2.lon - p1.lon) * (p2.lon - p1.lon) + (p2.lat - p1.lat) * (p2.lat - p1.lat);

    return (num * num / denom2) <= eps2;
}

static struct IntersectResult
lines_intersection(struct Line l1, struct Line l2, double eps)
{
    struct IntersectResult result = {.intersection = (struct Coord){.lat = NAN, .lon = NAN},
                                     .does_intersect = false,
                                     .intersect_is_endpoints = false,
                                     .msg = "nothing to report"};

    // Check if they are nearly co-linear
    unsigned int num_close = 0;
    if (line_coord_is_close(l1, l2.start, eps)) {
        ++num_close;
    }
    if (line_coord_is_close(l1, l2.end, eps)) {
        ++num_close;
    }
    if (line_coord_is_close(l2, l1.start, eps)) {
        ++num_close;
    }
    if (line_coord_is_close(l2, l1.end, eps)) {
        ++num_close;
    }
    if (num_close > 1) {
        result.does_intersect = false;
        result.msg = "colinear";
        return result;
    }

    double m1 = (l1.end.lat - l1.start.lat) / (l1.end.lon - l1.start.lon);
    double m2 = (l2.end.lat - l2.start.lat) / (l2.end.lon - l2.start.lon);

    double x1 = l1.start.lon;
    double y1 = l1.start.lat;
    double x2 = l2.start.lon;
    double y2 = l2.start.lat;

    if (m1 == m2 || (isinf(m1) && isinf(m2))) {
        // NOTE: This also captures colinear cases.
        result.does_intersect = false;
        result.msg = "parallel lines";
        return result;
    }

    double x0 = NAN;
    double y0 = NAN;
    if (isinf(m1)) {
        // l1 is vertical
        x0 = l1.start.lon;
        y0 = m2 * (x0 - x2) + y2;
    } else if (isinf(m2)) {
        // l2 is vertical
        x0 = l2.start.lon;
        y0 = m1 * (x0 - x1) + y1;
    } else {
        x0 = (y2 - y1 + m1 * x1 - m2 * x2) / (m1 - m2);
        y0 = m1 * (x0 - x1) + y1;
    }

    result.intersection = (struct Coord){.lat = y0, .lon = x0};
    struct Coord intersect = result.intersection; // short-hand

    if (y0 - fmax(l1.start.lat, l1.end.lat) > eps || fmin(l1.start.lat, l1.end.lat) - y0 > eps ||
        x0 - fmax(l1.start.lon, l1.end.lon) > eps || fmin(l1.start.lon, l1.end.lon) - x0 > eps) {
        // Test to make sure we are within the limits of l1

        result.does_intersect = false;
        result.msg = "intersection point outside line segment";
    } else if (y0 - fmax(l2.start.lat, l2.end.lat) > eps ||
               fmin(l2.start.lat, l2.end.lat) - y0 > eps ||
               x0 - fmax(l2.start.lon, l2.end.lon) > eps ||
               fmin(l2.start.lon, l2.end.lon) - x0 > eps) {
        // Test to make sure we are within the limits of l2

        result.does_intersect = false;
        result.msg = "intersection point outside line segment";
    } else {
        result.does_intersect = true;

        bool is_l1_endpoint =
            coord_are_close(intersect, l1.start, eps) || coord_are_close(intersect, l1.end, eps);

        bool is_l2_endpoint =
            coord_are_close(intersect, l2.start, eps) || coord_are_close(intersect, l2.end, eps);

        if (is_l1_endpoint && is_l2_endpoint) {
            result.intersect_is_endpoints = true;
        }
    }

    return result;
}

static struct Coord
triangle_centroid(struct Coord v1, struct Coord v2, struct Coord v3)
{
    double avg_lat = (v1.lat + v2.lat + v3.lat) / 3.0;
    double avg_lon = (v1.lon + v2.lon + v3.lon) / 3.0;

    return (struct Coord){.lat = avg_lat, .lon = avg_lon};
}

struct BoundingBox {
    struct Coord ll;
    struct Coord ur;
};

static bool
bounding_box_contains_coord(struct BoundingBox const box, struct Coord const coord, double eps)
{
    bool lon_in_range = (coord.lon - box.ur.lon) < eps && (coord.lon - box.ll.lon) > -eps;
    bool lat_in_range = (coord.lat - box.ur.lat) < eps && (coord.lat - box.ll.lat) > -eps;

    return lon_in_range && lat_in_range;
}

static bool
bounding_boxes_overlap(struct BoundingBox const left, struct BoundingBox const right, double eps)
{
    struct Coord right_coords[4] = {right.ll, right.ur,
                                    (struct Coord){.lat = right.ll.lat, .lon = right.ur.lon},
                                    (struct Coord){.lat = right.ur.lat, .lon = right.ll.lon}};

    struct Coord left_coords[4] = {left.ll, left.ur,
                                   (struct Coord){.lat = left.ll.lat, .lon = left.ur.lon},
                                   (struct Coord){.lat = left.ur.lat, .lon = left.ll.lon}};

    for (unsigned int i = 0; i < 4; ++i) {
        if (bounding_box_contains_coord(left, right_coords[i], eps)) {
            return true;
        }

        if (bounding_box_contains_coord(right, left_coords[i], eps)) {
            return true;
        }
    }

    return false;
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

static struct BoundingBox
sat_pixel_bounding_box(struct SatPixel const pxl[static 1])
{
    double xmax = fmax(pxl->ur.lon, pxl->lr.lon);
    double xmin = fmin(pxl->ul.lon, pxl->ll.lon);
    double ymax = fmax(pxl->ur.lat, pxl->ul.lat);
    double ymin = fmin(pxl->lr.lat, pxl->ll.lat);

    struct Coord ll = {.lat = ymin, .lon = xmin};
    struct Coord ur = {.lat = ymax, .lon = xmax};

    return (struct BoundingBox){.ll = ll, .ur = ur};
}

static bool
sat_pixels_bounding_boxes_overlap(struct SatPixel const left[static 1],
                                  struct SatPixel const right[static 1], double eps)
{
    struct BoundingBox bb_left = sat_pixel_bounding_box(left);
    struct BoundingBox bb_right = sat_pixel_bounding_box(right);

    return bounding_boxes_overlap(bb_left, bb_right, eps);
}

struct Coord
sat_pixel_centroid(struct SatPixel const pxl[static 1])
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

    struct IntersectResult res = lines_intersection(diag1_centroids, diag2_centroids, 1.0e-30);

    assert(res.does_intersect);

    return res.intersection;
}

bool
sat_pixels_approx_equal(struct SatPixel left[static 1], struct SatPixel right[static 1], double eps)
{
    return coord_are_close(left->ul, right->ul, eps) && coord_are_close(left->ur, right->ur, eps) &&
           coord_are_close(left->lr, right->lr, eps) && coord_are_close(left->ll, right->ll, eps);
}

bool
sat_pixel_contains_coord(struct SatPixel const pxl[static 1], struct Coord coord, double eps)
{
    // Check if it's outside the bounding box first. This is easy, and if it is,
    // then we already know the answer.
    struct BoundingBox const box = sat_pixel_bounding_box(pxl);

    if (!bounding_box_contains_coord(box, coord, eps)) {
        return false;
    }

    // Make a line from the point in question to each corner of the quadrilateral. If any of those
    // lines intersect an edge of the quadrilateral, the the point is outside. Note that the
    // line_intersection() function takes the eps argument and uses that to determine if the
    // intersection is near an end point. If it is, then we ignore it. So there is some fuzziness
    // to this function. If a coordinate outside the pixel is close enough to one of the edges,
    // it is possible it would be classified as inside. But it has to be eps close! And even then
    // it's not guaranteed.
    struct Line pxl_lines[4] = {
        (struct Line){.start = pxl->ul, .end = pxl->ur},
        (struct Line){.start = pxl->ur, .end = pxl->lr},
        (struct Line){.start = pxl->lr, .end = pxl->ll},
        (struct Line){.start = pxl->ll, .end = pxl->ul},
    };

    struct Line coord_lines[4] = {
        (struct Line){.start = coord, .end = pxl->ul},
        (struct Line){.start = coord, .end = pxl->ur},
        (struct Line){.start = coord, .end = pxl->ll},
        (struct Line){.start = coord, .end = pxl->lr},
    };

    for (unsigned int i = 0; i < 4; ++i) {
        for (unsigned int j = 0; j < 4; ++j) {
            struct IntersectResult res = lines_intersection(pxl_lines[i], coord_lines[j], eps);

            if (res.does_intersect && !res.intersect_is_endpoints) {
                return false;
            }
        }
    }

    return true;
}

bool
sat_pixels_overlap(struct SatPixel left[static 1], struct SatPixel right[static 1], double eps)
{
    // Check if they are equal first, then of course they overlap!
    if (sat_pixels_approx_equal(left, right, eps)) {
        return true;
    }

    // Check the bounding boxes.
    if (!sat_pixels_bounding_boxes_overlap(left, right, eps)) {
        return false;
    }

    // If pixels overlap, then at least 1 vertex from one pixel must be inside the boundary of
    // the other pixel or the pixels must have lines that intersect. In the case of one pixel
    // completely contained inside another (extremely unlikely), there would be no intersections
    // but all the points of one would be contained in another. In any other case, there must be
    // an intersection of lines.
    //
    // This is all by my own reasoning, not based on any math book or papers on geometry. I'm
    // assuming all pixels are convex quadrilaterals.

    // Check for intersecting lines between the pixels.
    struct Line left_pxl_lines[4] = {
        (struct Line){.start = left->ul, .end = left->ur},
        (struct Line){.start = left->ur, .end = left->lr},
        (struct Line){.start = left->lr, .end = left->ll},
        (struct Line){.start = left->ll, .end = left->ul},
    };

    struct Line right_pxl_lines[4] = {
        (struct Line){.start = right->ul, .end = right->ur},
        (struct Line){.start = right->ur, .end = right->lr},
        (struct Line){.start = right->lr, .end = right->ll},
        (struct Line){.start = right->ll, .end = right->ul},
    };

    for (unsigned i = 0; i < 4; ++i) {
        struct Line left = left_pxl_lines[i];

        for (unsigned j = 0; j < 4; ++j) {
            struct Line right = right_pxl_lines[j];

            struct IntersectResult res = lines_intersection(left, right, eps);

            if (res.does_intersect && !res.intersect_is_endpoints) {
                return true;
            }
        }
    }

    // Checking for intersecting lines didn't find anything. Now try seeing if one pixel is
    // contained in the other pixel.
    struct Coord left_coords[4] = {left->ul, left->ur, left->lr, left->ll};
    for (unsigned i = 0; i < 4; ++i) {
        if (sat_pixel_contains_coord(right, left_coords[i], eps)) {
            return true;
        }
    }

    struct Coord right_coords[4] = {right->ul, right->ur, right->lr, right->ll};
    for (unsigned i = 0; i < 4; ++i) {
        if (sat_pixel_contains_coord(left, right_coords[i], eps)) {
            return true;
        }
    }

    // No intersecting lines and no corners of one pixel contained in the other, so there
    // is no overlap.
    return false;
}

bool
sat_pixels_are_adjacent(struct SatPixel left[static 1], struct SatPixel right[static 1], double eps)
{
    if (sat_pixels_approx_equal(left, right, eps)) {
        return false;
    }

    if (!sat_pixels_bounding_boxes_overlap(left, right, eps)) {
        return false;
    }

    struct Coord left_coords[4] = {left->ul, left->ur, left->lr, left->ll};
    struct Coord right_coords[4] = {right->ul, right->ur, right->lr, right->ll};

    // Count the number of close coords and mark which ones are close.
    bool left_close[4] = {false, false, false, false};
    bool right_close[4] = {false, false, false, false};
    unsigned int num_close_coords = 0;
    for (unsigned int i = 0; i < 4; ++i) {
        for (unsigned int j = 0; j < 4; ++j) {
            if (coord_are_close(left_coords[i], right_coords[j], eps)) {
                ++num_close_coords;
                left_close[i] = true;
                right_close[j] = true;
            }
        }
    }

    // bail out early if we can
    if (num_close_coords < 1 || num_close_coords > 2) {
        return false;
    }

    // Check if any not close points are contained in the other pixel
    for (unsigned int i = 0; i < 4; ++i) {
        if (!left_close[i]) {
            if (sat_pixel_contains_coord(right, left_coords[i], eps)) {
                return false;
            }
        }

        if (!right_close[i]) {
            if (sat_pixel_contains_coord(left, right_coords[i], eps)) {
                return false;
            }
        }
    }

    // The following is a heuristic that should catch most of the remaining edge cases. For the
    // satellite data this program will be working with, this should really be more than good
    // enough.

    // If they are adjacent, the centroid of neither should be interior to the other.
    struct Coord left_centroid = sat_pixel_centroid(left);
    if (sat_pixel_contains_coord(right, left_centroid, eps)) {
        return false;
    }
    struct Coord right_centroid = sat_pixel_centroid(right);
    if (sat_pixel_contains_coord(left, right_centroid, eps)) {
        return false;
    }

    return true;
}

/*-------------------------------------------------------------------------------------------------
 *                                         PixelList
 *-----------------------------------------------------------------------------------------------*/
static struct PixelList *
pixel_list_expand(struct PixelList plist[static 1])
{
    size_t new_capacity = (plist->capacity * 3) / 2;

    plist = realloc(plist, sizeof(struct PixelList) + new_capacity * sizeof(struct SatPixel));
    Stopif(!plist, exit(EXIT_FAILURE), "unable to realloc, aborting");
    plist->capacity = new_capacity;

    return plist;
}

struct PixelList *
pixel_list_new()
{
    size_t const initial_capacity = 4;

    return pixel_list_new_with_capacity(initial_capacity);
}

struct PixelList *
pixel_list_new_with_capacity(size_t capacity)
{
    // We have to start at a minimal size of 2 for the 3/2 expansion factor to work (integer
    // aritmetic).
    if (capacity < 2) {
        capacity = 2;
    }

    struct PixelList *ptr =
        calloc(sizeof(struct PixelList) + capacity * sizeof(struct SatPixel), sizeof(char));

    Stopif(!ptr, exit(EXIT_FAILURE), "unable to calloc, aborting");

    ptr->capacity = capacity;

    assert(ptr->len == 0);
    return ptr;
}

struct PixelList *
pixel_list_destroy(struct PixelList plist[static 1])
{
    free(plist);
    return 0;
}

struct PixelList *
pixel_list_copy(struct PixelList plist[static 1])
{
    assert(plist);

    size_t copy_size = plist->len >= 4 ? plist->len : 4;
    struct PixelList *copy = pixel_list_new_with_capacity(copy_size);
    memcpy(copy, plist, sizeof(struct PixelList) + copy_size * sizeof(struct SatPixel));

    return copy;
}

struct PixelList *
pixel_list_append(struct PixelList list[static 1], struct SatPixel apix[static 1])
{
    if (list->len == list->capacity) {
        list = pixel_list_expand(list);
    }

    list->pixels[list->len] = *apix;
    list->len++;
    return list;
}

struct PixelList *
pixel_list_clear(struct PixelList list[static 1])
{
    list->len = 0;
    return list;
}

struct Coord
pixel_list_centroid(struct PixelList const list[static 1])
{
    assert(list);

    struct Coord centroid = {.lat = 0.0, .lon = 0.0};
    for (unsigned int i = 0; i < list->len; ++i) {
        struct Coord coord = sat_pixel_centroid(&list->pixels[i]);
        centroid.lat += coord.lat;
        centroid.lon += coord.lon;
    }

    centroid.lat /= (double)list->len;
    centroid.lon /= (double)list->len;

    return centroid;
}

/*-------------------------------------------------------------------------------------------------
 *                                         Binary Format
 *-----------------------------------------------------------------------------------------------*/
size_t
pixel_list_binary_serialize_buffer_size(struct PixelList const plist[static 1])
{
    return sizeof(struct PixelList) + sizeof(struct SatPixel) * plist->len;
}

size_t
pixel_list_binary_serialize(struct PixelList const plist[static 1], size_t buf_size,
                            unsigned char buffer[buf_size])
{
    memcpy(buffer, plist, buf_size);

    return buf_size;
}

struct PixelList *
pixel_list_binary_deserialize(unsigned char buffer[static sizeof(size_t)])
{
    // member len needs to be first for the current binary serialization scheme.
    size_t len = 0;
    memcpy(&len, buffer, sizeof(len));

    size_t buf_len = sizeof(struct PixelList) + sizeof(struct SatPixel) * len;

    struct PixelList *list = calloc(buf_len, sizeof(unsigned char));

    Stopif(!list, exit(EXIT_FAILURE), "out of memory, aborting");

    memcpy(list, buffer, buf_len);
    list->capacity = list->len;

    return list;
}

/*-------------------------------------------------------------------------------------------------
 *                                         KML Export
 *-----------------------------------------------------------------------------------------------*/
void
pixel_list_kml_write(FILE *strm, struct PixelList const plist[static 1])
{
    assert(plist);

    kml_start_multigeometry(strm);
    for (unsigned int i = 0; i < plist->len; ++i) {
        struct SatPixel pixel = plist->pixels[i];
        kml_start_polygon(strm);
        kml_polygon_start_outer_ring(strm);
        kml_start_linear_ring(strm);

        for (unsigned int j = 0; j < sizeof(pixel.coords) / sizeof(pixel.coords[0]); ++j) {
            struct Coord coord = pixel.coords[j];
            kml_linear_ring_add_vertex(strm, coord.lat, coord.lon);
        }
        // Close the loop.
        struct Coord coord = pixel.coords[0];
        kml_linear_ring_add_vertex(strm, coord.lat, coord.lon);

        kml_end_linear_ring(strm);
        kml_polygon_end_outer_ring(strm);
        kml_end_polygon(strm);
    }
    struct Coord centroid = pixel_list_centroid(plist);
    kml_point(strm, centroid.lat, centroid.lon);
    kml_end_multigeometry(strm);

    return;
}

