
#include <float.h>
#include <locale.h>
#include <stdbool.h>

#include <glib.h>

#include "../src/geo.h"

/*-------------------------------------------------------------------------------------------------
 *
 *                                      Tests for geo.c
 *
 *-----------------------------------------------------------------------------------------------*/

// ------------------------------- Tests for the Coord type ---------------------------------------
static void
test_coord_are_close(void)
{
    struct Coord left = {.lat = 45.5, .lon = -120.0};
    struct Coord right = {.lat = 45.5000002, .lon = -120.0000002};

    g_assert_true(coord_are_close(left, left, 1.0e-6));
    g_assert_true(coord_are_close(right, right, 1.0e-6));
    g_assert_true(coord_are_close(left, right, 1.0e-6));

    g_assert_false(coord_are_close(left, right, 1.0e-8));
}

// ----------------------------- Tests for the SatPixel type --------------------------------------

static void
test_sat_pixel_centroid(void)
{
    struct SatPixel pxl = {.ul = (struct Coord){.lat = 45.0, .lon = -120.0},
                           .ll = (struct Coord){.lat = 44.0, .lon = -120.0},
                           .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                           .ur = (struct Coord){.lat = 45.0, .lon = -119.0}};

    struct Coord centroid = {.lat = 44.5, .lon = -119.5};

    struct Coord centroid_calc = sat_pixel_centroid(&pxl);

    g_assert_true(coord_are_close(centroid, centroid_calc, 1.0e-12));
}

static void
test_sat_pixels_approx_equal()
{
    struct SatPixel pxl1 = {.ul = (struct Coord){.lat = 45.0, .lon = -120.0},
                            .ll = (struct Coord){.lat = 44.0, .lon = -120.0},
                            .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                            .ur = (struct Coord){.lat = 45.0, .lon = -119.0}};

    struct SatPixel pxl2 = {.ul = (struct Coord){.lat = 45.0000002, .lon = -120.0000002},
                            .ll = (struct Coord){.lat = 44.0000002, .lon = -119.9999998},
                            .lr = (struct Coord){.lat = 43.9999998, .lon = -119.0000002},
                            .ur = (struct Coord){.lat = 44.9999998, .lon = -118.9999998}};

    g_assert_true(sat_pixels_approx_equal(&pxl1, &pxl1, 1.0e-6));
    g_assert_true(sat_pixels_approx_equal(&pxl2, &pxl2, 1.0e-6));
    g_assert_true(sat_pixels_approx_equal(&pxl1, &pxl2, 1.0e-6));

    g_assert_false(sat_pixels_approx_equal(&pxl1, &pxl2, 1.0e-8));
}

static void
test_sat_pixel_contains_coord(void)
{
    // This is a simple square of width & height 1 degree of latitude & longitude
    struct SatPixel pxl1 = {.ul = (struct Coord){.lat = 45.0, .lon = -120.0},
                            .ll = (struct Coord){.lat = 44.0, .lon = -120.0},
                            .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                            .ur = (struct Coord){.lat = 45.0, .lon = -119.0}};

    struct Coord inside1 = {.lat = 44.5, .lon = -119.5};

    struct Coord outside1 = {.lat = 45.5, .lon = -119.5};
    struct Coord outside2 = {.lat = 44.5, .lon = -120.5};
    struct Coord outside3 = {.lat = 43.5, .lon = -119.5};
    struct Coord outside4 = {.lat = 44.5, .lon = -118.5};
    struct Coord outside5 = {.lat = 43.5, .lon = -118.5};
    struct Coord outside6 = {.lat = 45.5, .lon = -120.5};

    struct Coord boundary1 = {.lat = 45.0, .lon = -119.5};
    struct Coord boundary2 = {.lat = 44.0, .lon = -119.5};
    struct Coord boundary3 = {.lat = 44.5, .lon = -119.0};
    struct Coord boundary4 = {.lat = 44.5, .lon = -120.0};

    // Make sure what's inside is in
    g_assert_true(sat_pixel_contains_coord(&pxl1, inside1, 1.0e-6));

    // Make sure what's outside is out
    g_assert_false(sat_pixel_contains_coord(&pxl1, outside1, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl1, outside2, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl1, outside3, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl1, outside4, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl1, outside5, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl1, outside6, 1.0e-6));

    // Make sure what lies on the boundary is NOT contained in the polygon.
    g_assert_false(sat_pixel_contains_coord(&pxl1, boundary1, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl1, boundary2, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl1, boundary3, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl1, boundary4, 1.0e-6));

    // This is a very skewed quadrilateral
    struct SatPixel pxl2 = {.ul = (struct Coord){.lat = 3.0, .lon = 2.0},
                            .ll = (struct Coord){.lat = 0.0, .lon = 0.0},
                            .lr = (struct Coord){.lat = 2.0, .lon = 2.0},
                            .ur = (struct Coord){.lat = 5.0, .lon = 4.0}};

    inside1 = (struct Coord){.lat = 2.5, .lon = 2.0};

    outside1 = (struct Coord){.lat = 2.0, .lon = 1.0};
    outside2 = (struct Coord){.lat = 1.0, .lon = 2.0};
    outside3 = (struct Coord){.lat = -1.5, .lon = -119.5};

    boundary1 = (struct Coord){.lat = 1.0, .lon = 1.0};
    boundary2 = (struct Coord){.lat = 4.0, .lon = 3.0};

    // Make sure what's inside is in
    g_assert_true(sat_pixel_contains_coord(&pxl2, inside1, 1.0e-6));

    // Make sure what's outside is out
    g_assert_false(sat_pixel_contains_coord(&pxl2, outside1, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl2, outside2, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl2, outside3, 1.0e-6));

    // Make sure what lies on the boundary is NOT contained in the polygon.
    g_assert_false(sat_pixel_contains_coord(&pxl2, boundary1, 1.0e-6));
    g_assert_false(sat_pixel_contains_coord(&pxl2, boundary2, 1.0e-6));
}

static void
test_sat_pixels_overlap(void)
{
    struct SatPixel pxl1 = {.ul = (struct Coord){.lat = 45.0, .lon = -120.0},
                            .ll = (struct Coord){.lat = 44.0, .lon = -120.0},
                            .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                            .ur = (struct Coord){.lat = 45.0, .lon = -119.0}};

    struct SatPixel pxl2 = {.ul = (struct Coord){.lat = 45.5, .lon = -120.5},
                            .ll = (struct Coord){.lat = 44.5, .lon = -120.5},
                            .lr = (struct Coord){.lat = 44.5, .lon = -119.5},
                            .ur = (struct Coord){.lat = 45.5, .lon = -119.5}};

    struct SatPixel pxl3 = {.ul = (struct Coord){.lat = 46.0, .lon = -120.0},
                            .ll = (struct Coord){.lat = 45.0, .lon = -120.0},
                            .lr = (struct Coord){.lat = 45.0, .lon = -119.0},
                            .ur = (struct Coord){.lat = 46.0, .lon = -119.0}};

    // The corners of pxl4 lie along the mid-points of pxl1. So they overlap.
    struct SatPixel pxl4 = {.ul = (struct Coord){.lat = 45.0, .lon = -119.5},
                            .ll = (struct Coord){.lat = 44.5, .lon = -120.0},
                            .lr = (struct Coord){.lat = 44.0, .lon = -119.5},
                            .ur = (struct Coord){.lat = 44.5, .lon = -119.0}};

    // pixels are always overlapping themselves.
    g_assert_true(sat_pixels_overlap(&pxl1, &pxl1, 1.0e-6));
    g_assert_true(sat_pixels_overlap(&pxl2, &pxl2, 1.0e-6));
    g_assert_true(sat_pixels_overlap(&pxl3, &pxl3, 1.0e-6));
    g_assert_true(sat_pixels_overlap(&pxl4, &pxl4, 1.0e-6));

    // pxl1 and pxl3 are adjacent, but they do not overlap.
    g_assert_false(sat_pixels_overlap(&pxl1, &pxl3, 1.0e-6));
    g_assert_false(sat_pixels_overlap(&pxl3, &pxl1, 1.0e-6));

    // pxl2 overlaps pxl1 and pxl3 - order doesn't matter
    g_assert_true(sat_pixels_overlap(&pxl1, &pxl2, 1.0e-6));
    g_assert_true(sat_pixels_overlap(&pxl2, &pxl1, 1.0e-6));

    g_assert_true(sat_pixels_overlap(&pxl3, &pxl2, 1.0e-6));
    g_assert_true(sat_pixels_overlap(&pxl2, &pxl3, 1.0e-6));

    // Test the case where a vertex lies on the boundary.
    g_assert_true(sat_pixels_overlap(&pxl1, &pxl4, 1.0e-6));
    g_assert_true(sat_pixels_overlap(&pxl4, &pxl1, 1.0e-6));
}

static void
test_sat_pixels_are_adjacent(void)
{
    struct SatPixel pxl_nw = {.ul = (struct Coord){.lat = 46.0, .lon = -121.0},
                              .ll = (struct Coord){.lat = 45.0, .lon = -121.0},
                              .lr = (struct Coord){.lat = 45.0, .lon = -120.0},
                              .ur = (struct Coord){.lat = 46.0, .lon = -120.0}};

    struct SatPixel pxl_nn = {.ul = (struct Coord){.lat = 46.0, .lon = -120.0},
                              .ll = (struct Coord){.lat = 45.0, .lon = -120.0},
                              .lr = (struct Coord){.lat = 45.0, .lon = -119.0},
                              .ur = (struct Coord){.lat = 46.0, .lon = -119.0}};

    struct SatPixel pxl_ne = {.ul = (struct Coord){.lat = 46.0, .lon = -119.0},
                              .ll = (struct Coord){.lat = 45.0, .lon = -119.0},
                              .lr = (struct Coord){.lat = 45.0, .lon = -118.0},
                              .ur = (struct Coord){.lat = 46.0, .lon = -118.0}};

    struct SatPixel pxl_ww = {.ul = (struct Coord){.lat = 45.0000002, .lon = -121.0000002},
                              .ll = (struct Coord){.lat = 44.0000002, .lon = -120.9999998},
                              .lr = (struct Coord){.lat = 43.9999998, .lon = -120.0000002},
                              .ur = (struct Coord){.lat = 44.9999998, .lon = -119.9999998}};

    struct SatPixel pxl_00 = {.ul = (struct Coord){.lat = 45.0, .lon = -120.0},
                              .ll = (struct Coord){.lat = 44.0, .lon = -120.0},
                              .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                              .ur = (struct Coord){.lat = 45.0, .lon = -119.0}};

    struct SatPixel pxl_ee = {.ul = (struct Coord){.lat = 45.0, .lon = -119.0},
                              .ll = (struct Coord){.lat = 44.0, .lon = -119.0},
                              .lr = (struct Coord){.lat = 44.0, .lon = -118.0},
                              .ur = (struct Coord){.lat = 45.0, .lon = -118.0}};

    struct SatPixel pxl_sw = {.ul = (struct Coord){.lat = 44.0, .lon = -121.0},
                              .ll = (struct Coord){.lat = 43.0, .lon = -121.0},
                              .lr = (struct Coord){.lat = 43.0, .lon = -120.0},
                              .ur = (struct Coord){.lat = 44.0, .lon = -120.0}};

    struct SatPixel pxl_ss = {.ul = (struct Coord){.lat = 44.0, .lon = -120.0},
                              .ll = (struct Coord){.lat = 43.0, .lon = -120.0},
                              .lr = (struct Coord){.lat = 43.0, .lon = -119.0},
                              .ur = (struct Coord){.lat = 44.0, .lon = -119.0}};

    struct SatPixel pxl_se = {.ul = (struct Coord){.lat = 44.0, .lon = -119.0},
                              .ll = (struct Coord){.lat = 43.0, .lon = -119.0},
                              .lr = (struct Coord){.lat = 43.0, .lon = -118.0},
                              .ur = (struct Coord){.lat = 44.0, .lon = -118.0}};

    // Pixels are not adjacent to themselves.
    g_assert_false(sat_pixels_are_adjacent(&pxl_nw, &pxl_nw, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nn, &pxl_nn, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ne, &pxl_ne, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ww, &pxl_ww, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_00, &pxl_00, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ee, &pxl_ee, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sw, &pxl_sw, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ss, &pxl_ss, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_se, &pxl_se, 1.0e-6));

    // Check west-to-east (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_nw, &pxl_nn, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_ne, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nw, &pxl_ne, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ww, &pxl_00, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_ee, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ww, &pxl_ee, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_sw, &pxl_ss, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_se, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sw, &pxl_se, 1.0e-6));

    // Check east-to-west (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_ne, &pxl_nn, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_nw, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ne, &pxl_nw, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_00, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_ww, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ee, &pxl_ww, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_se, &pxl_ss, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_sw, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_se, &pxl_sw, 1.0e-6));

    // Check north-to-south (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_nw, &pxl_ww, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ww, &pxl_sw, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nw, &pxl_sw, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_00, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_ss, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nn, &pxl_ss, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ne, &pxl_ee, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_se, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ne, &pxl_se, 1.0e-6));

    // Check south-to-north (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_sw, &pxl_ww, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ww, &pxl_nw, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sw, &pxl_nw, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_00, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_nn, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ss, &pxl_nn, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_se, &pxl_ee, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_ne, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_se, &pxl_ne, 1.0e-6));

    // Check southwest-to-northeast and southeast-to-northwest (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_sw, &pxl_00, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_ne, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sw, &pxl_ne, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_se, &pxl_00, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_nw, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_se, &pxl_nw, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ww, &pxl_nn, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_ee, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_ww, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_nn, 1.0e-6));

    // Check northwest-to-southeast and northeast-to-southwest (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_nw, &pxl_00, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_se, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nw, &pxl_se, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ne, &pxl_00, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_sw, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ne, &pxl_sw, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_ww, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_ss, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ww, &pxl_ss, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_ee, 1.0e-6));

    //
    // Check to make sure eps is working.
    //
    g_assert_true(sat_pixels_are_adjacent(&pxl_nw, &pxl_nn, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_ne, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nw, &pxl_ne, 1.0e-8));

    // should overlap - but not adjacent
    g_assert_false(sat_pixels_are_adjacent(&pxl_ww, &pxl_00, 1.0e-8));
    g_assert_true(sat_pixels_overlap(&pxl_ww, &pxl_00, 1.0e-8));

    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_ee, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ww, &pxl_ee, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_sw, &pxl_ss, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_se, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sw, &pxl_se, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ne, &pxl_nn, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_nw, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ne, &pxl_nw, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_00, 1.0e-8));

    // should overlap
    g_assert_false(sat_pixels_are_adjacent(&pxl_00, &pxl_ww, 1.0e-8));
    g_assert_true(sat_pixels_overlap(&pxl_00, &pxl_ww, 1.0e-8));

    g_assert_false(sat_pixels_are_adjacent(&pxl_ee, &pxl_ww, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_se, &pxl_ss, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_sw, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_se, &pxl_sw, 1.0e-8));

    // should overlap
    g_assert_false(sat_pixels_are_adjacent(&pxl_nw, &pxl_ww, 1.0e-8));
    g_assert_true(sat_pixels_overlap(&pxl_nw, &pxl_ww, 1.0e-8));

    // should overlap
    g_assert_false(sat_pixels_are_adjacent(&pxl_ww, &pxl_sw, 1.0e-8));
    g_assert_true(sat_pixels_overlap(&pxl_ww, &pxl_sw, 1.0e-8));

    g_assert_false(sat_pixels_are_adjacent(&pxl_nw, &pxl_sw, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_00, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_ss, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nn, &pxl_ss, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ne, &pxl_ee, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_se, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ne, &pxl_se, 1.0e-8));

    // should overlap
    g_assert_false(sat_pixels_are_adjacent(&pxl_sw, &pxl_ww, 1.0e-8));
    g_assert_true(sat_pixels_overlap(&pxl_sw, &pxl_ww, 1.0e-8));

    // should overlap
    g_assert_false(sat_pixels_are_adjacent(&pxl_ww, &pxl_nw, 1.0e-8));
    g_assert_true(sat_pixels_overlap(&pxl_ww, &pxl_sw, 1.0e-8));

    g_assert_false(sat_pixels_are_adjacent(&pxl_sw, &pxl_nw, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_00, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_nn, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ss, &pxl_nn, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_se, &pxl_ee, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_ne, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_se, &pxl_ne, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_sw, &pxl_00, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_ne, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sw, &pxl_ne, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_se, &pxl_00, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_nw, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_se, &pxl_nw, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nw, &pxl_00, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_se, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nw, &pxl_se, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_ne, &pxl_00, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_00, &pxl_sw, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_ne, &pxl_sw, 1.0e-8));

    // should be false
    g_assert_false(sat_pixels_are_adjacent(&pxl_ww, &pxl_nn, 1.0e-8));

    g_assert_true(sat_pixels_are_adjacent(&pxl_ss, &pxl_ee, 1.0e-8));

    // should be false
    g_assert_false(sat_pixels_are_adjacent(&pxl_ss, &pxl_ww, 1.0e-8));

    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_nn, 1.0e-8));

    // should be false
    g_assert_false(sat_pixels_are_adjacent(&pxl_nn, &pxl_ww, 1.0e-8));

    g_assert_true(sat_pixels_are_adjacent(&pxl_ee, &pxl_ss, 1.0e-8));

    // should be false
    g_assert_false(sat_pixels_are_adjacent(&pxl_ww, &pxl_ss, 1.0e-8));

    g_assert_true(sat_pixels_are_adjacent(&pxl_nn, &pxl_ee, 1.0e-8));

    // Checking that there is no overlap is not good enough since there may be some overlap due to
    // using the eps variable to make the matching fuzzy. We should also check to make sure that
    // any vertices that aren't close aren't contained inside the other pixel.

    // This pixel is inside pxl_00, but it shares a common lower right corner
    struct SatPixel sub_pxl_01 = {.ul = (struct Coord){.lat = 44.5, .lon = -119.5},
                                  .ll = (struct Coord){.lat = 44.0, .lon = -119.5},
                                  .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                                  .ur = (struct Coord){.lat = 44.5, .lon = -119.0}};

    g_assert_false(sat_pixels_are_adjacent(&pxl_00, &sub_pxl_01, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&sub_pxl_01, &pxl_00, 1.0e-6));

    // This pixel overlaps pxl_00 and shares a right edge. These overlap, but aren't adjacent.
    struct SatPixel sub_pxl_02 = {.ul = (struct Coord){.lat = 45.0, .lon = -119.5},
                                  .ll = (struct Coord){.lat = 44.0, .lon = -119.5},
                                  .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                                  .ur = (struct Coord){.lat = 45.0, .lon = -119.0}};

    g_assert_false(sat_pixels_are_adjacent(&pxl_00, &sub_pxl_02, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&sub_pxl_02, &pxl_00, 1.0e-6));
}

// ------------------------------ Tests for PixelList Serialization -------------------------------
struct PixelListFixture {
    struct PixelList *list;
};

static void
pixel_list_test_setup(struct PixelListFixture fixture[static 1], gconstpointer _unused)
{
    struct SatPixel pixels[9] = {{.ul = (struct Coord){.lat = 46.0, .lon = -121.0},
                                  .ll = (struct Coord){.lat = 45.0, .lon = -121.0},
                                  .lr = (struct Coord){.lat = 45.0, .lon = -120.0},
                                  .ur = (struct Coord){.lat = 46.0, .lon = -120.0}},

                                 {.ul = (struct Coord){.lat = 46.0, .lon = -120.0},
                                  .ll = (struct Coord){.lat = 45.0, .lon = -120.0},
                                  .lr = (struct Coord){.lat = 45.0, .lon = -119.0},
                                  .ur = (struct Coord){.lat = 46.0, .lon = -119.0}},

                                 {.ul = (struct Coord){.lat = 46.0, .lon = -119.0},
                                  .ll = (struct Coord){.lat = 45.0, .lon = -119.0},
                                  .lr = (struct Coord){.lat = 45.0, .lon = -118.0},
                                  .ur = (struct Coord){.lat = 46.0, .lon = -118.0}},

                                 {.ul = (struct Coord){.lat = 45.0000002, .lon = -121.0000002},
                                  .ll = (struct Coord){.lat = 44.0000002, .lon = -120.9999998},
                                  .lr = (struct Coord){.lat = 43.9999998, .lon = -120.0000002},
                                  .ur = (struct Coord){.lat = 44.9999998, .lon = -119.9999998}},

                                 {.ul = (struct Coord){.lat = 45.0, .lon = -120.0},
                                  .ll = (struct Coord){.lat = 44.0, .lon = -120.0},
                                  .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                                  .ur = (struct Coord){.lat = 45.0, .lon = -119.0}},

                                 {.ul = (struct Coord){.lat = 45.0, .lon = -119.0},
                                  .ll = (struct Coord){.lat = 44.0, .lon = -119.0},
                                  .lr = (struct Coord){.lat = 44.0, .lon = -118.0},
                                  .ur = (struct Coord){.lat = 45.0, .lon = -118.0}},

                                 {.ul = (struct Coord){.lat = 44.0, .lon = -121.0},
                                  .ll = (struct Coord){.lat = 43.0, .lon = -121.0},
                                  .lr = (struct Coord){.lat = 43.0, .lon = -120.0},
                                  .ur = (struct Coord){.lat = 44.0, .lon = -120.0}},

                                 {.ul = (struct Coord){.lat = 44.0, .lon = -120.0},
                                  .ll = (struct Coord){.lat = 43.0, .lon = -120.0},
                                  .lr = (struct Coord){.lat = 43.0, .lon = -119.0},
                                  .ur = (struct Coord){.lat = 44.0, .lon = -119.0}},

                                 {.ul = (struct Coord){.lat = 44.0, .lon = -119.0},
                                  .ll = (struct Coord){.lat = 43.0, .lon = -119.0},
                                  .lr = (struct Coord){.lat = 43.0, .lon = -118.0},
                                  .ur = (struct Coord){.lat = 44.0, .lon = -118.0}}};

    fixture->list = pixel_list_new();

    for (unsigned int i = 0; i < sizeof(pixels) / sizeof(pixels[0]); ++i) {
        fixture->list = pixel_list_append(fixture->list, &pixels[i]);
    }
}

static void
pixel_list_test_teaddown(struct PixelListFixture fixture[static 1], gconstpointer _unused)
{
    fixture->list = pixel_list_destroy(fixture->list);
}

static void
pixel_list_test_binary_round_trip(struct PixelListFixture fixture[static 1], gconstpointer _unused)
{
    g_assert_cmpint(fixture->list->len, ==, 9);

    size_t buf_size = pixel_list_binary_serialize_buffer_size(fixture->list);

    unsigned char *buffer = 0;
    buffer = calloc(buf_size, sizeof(unsigned char));
    g_assert_true(buffer);

    size_t num_bytes_written = pixel_list_binary_serialize(fixture->list, buf_size, buffer);

    g_assert_cmpint(buf_size, ==, num_bytes_written);

    struct PixelList *decoded = pixel_list_binary_deserialize(buffer);

    g_assert_true(decoded);
    g_assert_cmpint(fixture->list->len, ==, decoded->len);

    for (unsigned int i = 0; i < decoded->len; ++i) {
        g_assert_true(
            sat_pixels_approx_equal(&fixture->list->pixels[i], &decoded->pixels[i], DBL_MIN));
    }

    free(buffer);
    decoded = pixel_list_destroy(decoded);
}
/*-------------------------------------------------------------------------------------------------
 *
 *                                      Main Test Runner
 *
 *-----------------------------------------------------------------------------------------------*/
int
main(int argc, char *argv[static 1])
{
    setlocale(LC_ALL, "");

    g_test_init(&argc, &argv, NULL);

    //
    // geo.c
    //

    // Coord
    g_test_add_func("/geo/coordinates/coord_are_close", test_coord_are_close);

    // SatPixel
    g_test_add_func("/geo/sat_pixel/sat_pixel_centroid", test_sat_pixel_centroid);
    g_test_add_func("/geo/sat_pixel/sat_pixels_approx_equal", test_sat_pixels_approx_equal);
    g_test_add_func("/geo/sat_pixel/sat_pixel_contains_coord", test_sat_pixel_contains_coord);
    g_test_add_func("/geo/sat_pixel/sat_pixels_overlap", test_sat_pixels_overlap);
    g_test_add_func("/geo/sat_pixel/sat_pixels_are_adjacent", test_sat_pixels_are_adjacent);

    // PixelList
    g_test_add("/geo/pixel_list/pixel_list_test_binary_round_trip", struct PixelListFixture, 0,
               pixel_list_test_setup, pixel_list_test_binary_round_trip, pixel_list_test_teaddown);

    //
    // Run tests
    //
    return g_test_run();
}
