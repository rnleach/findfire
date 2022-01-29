/** A coordinate consisting of a latitude and a longitude. */
pub struct Coord {
    /// Latitude. Should be -90 to 90, but that's not checked or enforced.
    pub lat: f64,
    /// Longitude. Should be -180 to 180, but that's not checked or enforced.
    pub lon: f64,
}

/// Represents a "square" area in latitude-longitude coordinates.
pub struct BoundingBox {
    /// The lower left corner of the box.  
    pub ll: Coord,
    /// The upper right corner of the box.
    pub ur: Coord,
}

pub trait Geo {
    fn centroid(&self) -> Coord;
    fn bounding_boxe(&self) -> BoundingBox;
}

/*
#include "sf_util.h"

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <tgmath.h>

#include "satfire.h"

#include "kamel.h"

/*-------------------------------------------------------------------------------------------------
 *                                    Helper types and functions
 *-----------------------------------------------------------------------------------------------*/
struct Line {
    struct SFCoord start;
    struct SFCoord end;
};

struct IntersectResult {
    struct SFCoord intersection;
    char const *msg;
    bool does_intersect;
    bool intersect_is_endpoints;
};

static bool
line_coord_is_close(struct Line const line, struct SFCoord const coord, double eps)
{
    struct SFCoord p0 = coord;
    struct SFCoord p1 = line.start;
    struct SFCoord p2 = line.end;
    double eps2 = eps * eps;

    double num = (p2.lon - p1.lon) * (p1.lat - p0.lat) - (p1.lon - p0.lon) * (p2.lat - p1.lat);
    double denom2 = (p2.lon - p1.lon) * (p2.lon - p1.lon) + (p2.lat - p1.lat) * (p2.lat - p1.lat);

    return (num * num / denom2) <= eps2;
}

static struct IntersectResult
lines_intersection(struct Line l1, struct Line l2, double eps)
{
    struct IntersectResult result = {.intersection = (struct SFCoord){.lat = NAN, .lon = NAN},
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

    result.intersection = (struct SFCoord){.lat = y0, .lon = x0};
    struct SFCoord intersect = result.intersection; // short-hand

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

        bool is_l1_endpoint = satfire_coord_are_close(intersect, l1.start, eps) ||
                              satfire_coord_are_close(intersect, l1.end, eps);

        bool is_l2_endpoint = satfire_coord_are_close(intersect, l2.start, eps) ||
                              satfire_coord_are_close(intersect, l2.end, eps);

        if (is_l1_endpoint && is_l2_endpoint) {
            result.intersect_is_endpoints = true;
        }
    }

    return result;
}

static struct SFCoord
triangle_centroid(struct SFCoord v1, struct SFCoord v2, struct SFCoord v3)
{
    double avg_lat = (v1.lat + v2.lat + v3.lat) / 3.0;
    double avg_lon = (v1.lon + v2.lon + v3.lon) / 3.0;

    return (struct SFCoord){.lat = avg_lat, .lon = avg_lon};
}

/*-------------------------------------------------------------------------------------------------
 *                                       BoundingBox
 *-----------------------------------------------------------------------------------------------*/
/** \brief Check to see if a Coord is inside of a BoundingBox.
 *
 * \param box is the bounding box in question.
 * \param coord is the coordinate, or point, in question.
 * \param eps is a fuzzy factor. Any point 'eps' close to the box will be considered internal as
 * well. If \a eps is 0.0, then the comparison is exact.
 *
 * \returns \c true if the point \a coord is interior to the box.
 */
bool
satfire_bounding_box_contains_coord(struct SFBoundingBox const box, struct SFCoord const coord,
                                    double eps)
{
    bool lon_in_range = (coord.lon - box.ur.lon) < eps && (coord.lon - box.ll.lon) > -eps;
    bool lat_in_range = (coord.lat - box.ur.lat) < eps && (coord.lat - box.ll.lat) > -eps;

    return lon_in_range && lat_in_range;
}

/** \brief Check to see if these bounding boxes overlap. */
bool
satfire_bounding_boxes_overlap(struct SFBoundingBox const *left, struct SFBoundingBox const *right,
                               double eps)
{
    assert(left);
    assert(right);

    if (satfire_bounding_box_contains_coord(*left, right->ll, eps) ||
        satfire_bounding_box_contains_coord(*left, right->ur, eps)) {
        return true;
    }

    if (satfire_bounding_box_contains_coord(*right, left->ll, eps) ||
        satfire_bounding_box_contains_coord(*right, left->ur, eps)) {
        return true;
    }

    return false;
}

static bool
bounding_boxes_overlap(struct SFBoundingBox const left, struct SFBoundingBox const right,
                       double eps)
{
    struct SFCoord right_coords[4] = {right.ll, right.ur,
                                      (struct SFCoord){.lat = right.ll.lat, .lon = right.ur.lon},
                                      (struct SFCoord){.lat = right.ur.lat, .lon = right.ll.lon}};

    struct SFCoord left_coords[4] = {left.ll, left.ur,
                                     (struct SFCoord){.lat = left.ll.lat, .lon = left.ur.lon},
                                     (struct SFCoord){.lat = left.ur.lat, .lon = left.ll.lon}};

    for (unsigned int i = 0; i < 4; ++i) {
        if (satfire_bounding_box_contains_coord(left, right_coords[i], eps)) {
            return true;
        }

        if (satfire_bounding_box_contains_coord(right, left_coords[i], eps)) {
            return true;
        }
    }

    return false;
}
/*-------------------------------------------------------------------------------------------------
 *                                         Coordinates
 *-----------------------------------------------------------------------------------------------*/
/** Determine if these coordinates are close to each other.
 *
 * The \a eps parameter is the maximum distance between points in the same units as the
 * coordinates that two points can have and still be considered close.
 */
bool
satfire_coord_are_close(struct SFCoord left, struct SFCoord right, double eps)
{
    double lat_diff = left.lat - right.lat;
    double lon_diff = left.lon - right.lon;
    double distance_squared = lat_diff * lat_diff + lon_diff * lon_diff;

    return distance_squared <= (eps * eps);
}
*/
