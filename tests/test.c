
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
    g_assert_true(sat_pixel_contains_coord(&pxl1, &inside1));

    // Make sure what's outside is out
    g_assert_false(sat_pixel_contains_coord(&pxl1, &outside1));
    g_assert_false(sat_pixel_contains_coord(&pxl1, &outside2));
    g_assert_false(sat_pixel_contains_coord(&pxl1, &outside3));
    g_assert_false(sat_pixel_contains_coord(&pxl1, &outside4));
    g_assert_false(sat_pixel_contains_coord(&pxl1, &outside5));
    g_assert_false(sat_pixel_contains_coord(&pxl1, &outside6));

    // Make sure what lies on the boundary is NOT contained in the polygon.
    g_assert_false(sat_pixel_contains_coord(&pxl1, &boundary1));
    g_assert_false(sat_pixel_contains_coord(&pxl1, &boundary2));
    g_assert_false(sat_pixel_contains_coord(&pxl1, &boundary3));
    g_assert_false(sat_pixel_contains_coord(&pxl1, &boundary4));

    // TODO: This only tests a simple square. We also need to test a skewed quadrilateral since
    // the further away from nadir you get, the more skewed the pixels are when projected onto the
    // earth's surface.
    g_assert_true(false);
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

    // pixels are always overlapping them selves.
    g_assert_true(sat_pixels_overlap(&pxl1, &pxl1));
    g_assert_true(sat_pixels_overlap(&pxl2, &pxl2));
    g_assert_true(sat_pixels_overlap(&pxl3, &pxl3));

    // pxl1 and pxl3 are adjacent, but they do not overlap.
    g_assert_false(sat_pixels_overlap(&pxl1, &pxl3));
    g_assert_false(sat_pixels_overlap(&pxl3, &pxl1));

    // pxl2 overlaps pxl1 and pxl3 - order doesn't matter
    g_assert_true(sat_pixels_overlap(&pxl1, &pxl2));
    g_assert_true(sat_pixels_overlap(&pxl2, &pxl1));

    g_assert_true(sat_pixels_overlap(&pxl3, &pxl2));
    g_assert_true(sat_pixels_overlap(&pxl2, &pxl3));

    // TODO: Test the case where a vertex lies on the boundary.
    g_assert_true(false);
}

static void
test_sat_pixels_are_adjacent(void)
{
    struct SatPixel pxl_nth = {.ul = (struct Coord){.lat = 46.0, .lon = -120.0},
                               .ll = (struct Coord){.lat = 45.0, .lon = -120.0},
                               .lr = (struct Coord){.lat = 45.0, .lon = -119.0},
                               .ur = (struct Coord){.lat = 46.0, .lon = -119.0}};

    struct SatPixel pxl_wst = {.ul = (struct Coord){.lat = 45.0000002, .lon = -121.0000002},
                               .ll = (struct Coord){.lat = 44.0000002, .lon = -120.9999998},
                               .lr = (struct Coord){.lat = 43.9999998, .lon = -120.0000002},
                               .ur = (struct Coord){.lat = 44.9999998, .lon = -119.9999998}};

    struct SatPixel pxl_mid = {.ul = (struct Coord){.lat = 45.0, .lon = -120.0},
                               .ll = (struct Coord){.lat = 44.0, .lon = -120.0},
                               .lr = (struct Coord){.lat = 44.0, .lon = -119.0},
                               .ur = (struct Coord){.lat = 45.0, .lon = -119.0}};

    struct SatPixel pxl_est = {.ul = (struct Coord){.lat = 45.0, .lon = -119.0},
                               .ll = (struct Coord){.lat = 44.0, .lon = -119.0},
                               .lr = (struct Coord){.lat = 44.0, .lon = -118.0},
                               .ur = (struct Coord){.lat = 45.0, .lon = -118.0}};

    struct SatPixel pxl_sth = {.ul = (struct Coord){.lat = 44.0, .lon = -120.0},
                               .ll = (struct Coord){.lat = 43.0, .lon = -120.0},
                               .lr = (struct Coord){.lat = 43.0, .lon = -119.0},
                               .ur = (struct Coord){.lat = 44.0, .lon = -119.0}};

    // Pixels are not adjacent to themselves.
    g_assert_false(sat_pixels_are_adjacent(&pxl_wst, &pxl_wst, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_mid, &pxl_mid, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_est, &pxl_est, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nth, &pxl_nth, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sth, &pxl_sth, 1.0e-6));

    // Check west-to-east (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_wst, &pxl_mid, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_mid, &pxl_est, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_wst, &pxl_est, 1.0e-6));

    // Check east-to-west (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_est, &pxl_mid, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_mid, &pxl_wst, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_est, &pxl_wst, 1.0e-6));

    // Check north-to-south (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_nth, &pxl_mid, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_mid, &pxl_sth, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nth, &pxl_sth, 1.0e-6));

    // Check south-to-north (order shouldn't matter!)
    g_assert_true(sat_pixels_are_adjacent(&pxl_sth, &pxl_mid, 1.0e-6));
    g_assert_true(sat_pixels_are_adjacent(&pxl_mid, &pxl_nth, 1.0e-6));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sth, &pxl_nth, 1.0e-6));

    // Check to make sure eps is working.
    g_assert_false(sat_pixels_are_adjacent(&pxl_wst, &pxl_mid, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_mid, &pxl_est, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_wst, &pxl_est, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_est, &pxl_mid, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_mid, &pxl_wst, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_est, &pxl_wst, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_nth, &pxl_mid, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_mid, &pxl_sth, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_nth, &pxl_sth, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_sth, &pxl_mid, 1.0e-8));
    g_assert_true(sat_pixels_are_adjacent(&pxl_mid, &pxl_nth, 1.0e-8));
    g_assert_false(sat_pixels_are_adjacent(&pxl_sth, &pxl_nth, 1.0e-8));

    // TODO: I haven't tested the case where adjacent pixels share a single corner.
    g_assert_true(false);
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

    //
    // Run tests
    //
    return g_test_run();
}
