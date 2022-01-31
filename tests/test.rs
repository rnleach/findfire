/*
// ----------------------------- Tests for the SatPixel type --------------------------------------
static void
test_satfire_pixels_are_adjacent(void)
{
    struct SFPixel pxl_nw = {.ul = (struct SFCoord){.lat = 46.0, .lon = -121.0},
                             .ll = (struct SFCoord){.lat = 45.0, .lon = -121.0},
                             .lr = (struct SFCoord){.lat = 45.0, .lon = -120.0},
                             .ur = (struct SFCoord){.lat = 46.0, .lon = -120.0}};

    struct SFPixel pxl_nn = {.ul = (struct SFCoord){.lat = 46.0, .lon = -120.0},
                             .ll = (struct SFCoord){.lat = 45.0, .lon = -120.0},
                             .lr = (struct SFCoord){.lat = 45.0, .lon = -119.0},
                             .ur = (struct SFCoord){.lat = 46.0, .lon = -119.0}};

    struct SFPixel pxl_ne = {.ul = (struct SFCoord){.lat = 46.0, .lon = -119.0},
                             .ll = (struct SFCoord){.lat = 45.0, .lon = -119.0},
                             .lr = (struct SFCoord){.lat = 45.0, .lon = -118.0},
                             .ur = (struct SFCoord){.lat = 46.0, .lon = -118.0}};

    struct SFPixel pxl_ww = {.ul = (struct SFCoord){.lat = 45.0000002, .lon = -121.0000002},
                             .ll = (struct SFCoord){.lat = 44.0000002, .lon = -120.9999998},
                             .lr = (struct SFCoord){.lat = 43.9999998, .lon = -120.0000002},
                             .ur = (struct SFCoord){.lat = 44.9999998, .lon = -119.9999998}};

    struct SFPixel pxl_00 = {.ul = (struct SFCoord){.lat = 45.0, .lon = -120.0},
                             .ll = (struct SFCoord){.lat = 44.0, .lon = -120.0},
                             .lr = (struct SFCoord){.lat = 44.0, .lon = -119.0},
                             .ur = (struct SFCoord){.lat = 45.0, .lon = -119.0}};

    struct SFPixel pxl_ee = {.ul = (struct SFCoord){.lat = 45.0, .lon = -119.0},
                             .ll = (struct SFCoord){.lat = 44.0, .lon = -119.0},
                             .lr = (struct SFCoord){.lat = 44.0, .lon = -118.0},
                             .ur = (struct SFCoord){.lat = 45.0, .lon = -118.0}};

    struct SFPixel pxl_sw = {.ul = (struct SFCoord){.lat = 44.0, .lon = -121.0},
                             .ll = (struct SFCoord){.lat = 43.0, .lon = -121.0},
                             .lr = (struct SFCoord){.lat = 43.0, .lon = -120.0},
                             .ur = (struct SFCoord){.lat = 44.0, .lon = -120.0}};

    struct SFPixel pxl_ss = {.ul = (struct SFCoord){.lat = 44.0, .lon = -120.0},
                             .ll = (struct SFCoord){.lat = 43.0, .lon = -120.0},
                             .lr = (struct SFCoord){.lat = 43.0, .lon = -119.0},
                             .ur = (struct SFCoord){.lat = 44.0, .lon = -119.0}};

    struct SFPixel pxl_se = {.ul = (struct SFCoord){.lat = 44.0, .lon = -119.0},
                             .ll = (struct SFCoord){.lat = 43.0, .lon = -119.0},
                             .lr = (struct SFCoord){.lat = 43.0, .lon = -118.0},
                             .ur = (struct SFCoord){.lat = 44.0, .lon = -118.0}};

    // Pixels are not adjacent to themselves.
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nw, &pxl_nw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nn, &pxl_nn, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ne, &pxl_ne, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ww, &pxl_ww, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_00, &pxl_00, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ee, &pxl_ee, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_sw, &pxl_sw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ss, &pxl_ss, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_se, &pxl_se, 1.0e-6));

    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_nw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nn, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ne, &pxl_ne, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ww, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ee, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_sw, &pxl_sw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ss, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_se, &pxl_se, 1.0e-6));

    // Check west-to-east (order shouldn't matter!)
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nw, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_ne, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nw, &pxl_ne, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ww, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_ee, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ww, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_sw, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_se, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_sw, &pxl_se, 1.0e-6));

    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nn, &pxl_ne, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_ne, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ww, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_ee, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_ww, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_sw, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ss, &pxl_se, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_sw, &pxl_se, 1.0e-6));

    // Check east-to-west (order shouldn't matter!)
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ne, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_nw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ne, &pxl_nw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_ww, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ee, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_se, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_sw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_se, &pxl_sw, 1.0e-6));

    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ne, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nn, &pxl_nw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_ne, &pxl_nw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ee, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_ww, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_ee, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_se, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ss, &pxl_sw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_se, &pxl_sw, 1.0e-6));

    // Check north-to-south (order shouldn't matter!)
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nw, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ww, &pxl_sw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nw, &pxl_sw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_ss, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nn, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ne, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_se, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ne, &pxl_se, 1.0e-6));

    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ww, &pxl_sw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_sw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nn, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_ss, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_nn, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ne, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ee, &pxl_se, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_ne, &pxl_se, 1.0e-6));

    // Check south-to-north (order shouldn't matter!)
    g_assert_true(satfire_pixels_are_adjacent(&pxl_sw, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ww, &pxl_nw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_sw, &pxl_nw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_nn, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ss, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_se, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_ne, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_se, &pxl_ne, 1.0e-6));

    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_sw, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ww, &pxl_nw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_sw, &pxl_nw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ss, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_nn, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_ss, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_se, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ee, &pxl_ne, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_se, &pxl_ne, 1.0e-6));

    // Check southwest-to-northeast and southeast-to-northwest (order shouldn't matter!)
    g_assert_true(satfire_pixels_are_adjacent(&pxl_sw, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_ne, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_sw, &pxl_ne, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_se, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_nw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_se, &pxl_nw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ww, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_nn, 1.0e-6));

    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_sw, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_ne, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_sw, &pxl_ne, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_se, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_nw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_se, &pxl_nw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ww, &pxl_nn, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ss, &pxl_ee, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ss, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ee, &pxl_nn, 1.0e-6));

    // Check northwest-to-southeast and northeast-to-southwest (order shouldn't matter!)
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nw, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_se, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nw, &pxl_se, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ne, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_sw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ne, &pxl_sw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ww, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_ee, 1.0e-6));

    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_se, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_se, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ne, &pxl_00, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_sw, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_ne, &pxl_sw, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nn, &pxl_ww, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ee, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ww, &pxl_ss, 1.0e-6));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nn, &pxl_ee, 1.0e-6));

    //
    // Check to make sure eps is working.
    //
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nw, &pxl_nn, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_ne, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nw, &pxl_ne, 1.0e-8));

    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_nn, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_nn, &pxl_ne, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent_or_overlap(&pxl_nw, &pxl_ne, 1.0e-8));

    // should overlap - but not adjacent
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ww, &pxl_00, 1.0e-8));
    g_assert_true(satfire_pixels_overlap(&pxl_ww, &pxl_00, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_ww, &pxl_00, 1.0e-8));

    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_ee, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ww, &pxl_ee, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_sw, &pxl_ss, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_se, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_sw, &pxl_se, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ne, &pxl_nn, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_nw, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ne, &pxl_nw, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_00, 1.0e-8));

    // should overlap
    g_assert_false(satfire_pixels_are_adjacent(&pxl_00, &pxl_ww, 1.0e-8));
    g_assert_true(satfire_pixels_overlap(&pxl_00, &pxl_ww, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent_or_overlap(&pxl_00, &pxl_ww, 1.0e-8));

    g_assert_false(satfire_pixels_are_adjacent(&pxl_ee, &pxl_ww, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_se, &pxl_ss, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_sw, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_se, &pxl_sw, 1.0e-8));

    // should overlap
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nw, &pxl_ww, 1.0e-8));
    g_assert_true(satfire_pixels_overlap(&pxl_nw, &pxl_ww, 1.0e-8));

    // should overlap
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ww, &pxl_sw, 1.0e-8));
    g_assert_true(satfire_pixels_overlap(&pxl_ww, &pxl_sw, 1.0e-8));

    g_assert_false(satfire_pixels_are_adjacent(&pxl_nw, &pxl_sw, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_00, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_ss, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nn, &pxl_ss, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ne, &pxl_ee, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_se, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ne, &pxl_se, 1.0e-8));

    // should overlap
    g_assert_false(satfire_pixels_are_adjacent(&pxl_sw, &pxl_ww, 1.0e-8));
    g_assert_true(satfire_pixels_overlap(&pxl_sw, &pxl_ww, 1.0e-8));

    // should overlap
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ww, &pxl_nw, 1.0e-8));
    g_assert_true(satfire_pixels_overlap(&pxl_ww, &pxl_sw, 1.0e-8));

    g_assert_false(satfire_pixels_are_adjacent(&pxl_sw, &pxl_nw, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_00, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_nn, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ss, &pxl_nn, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_se, &pxl_ee, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_ne, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_se, &pxl_ne, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_sw, &pxl_00, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_ne, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_sw, &pxl_ne, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_se, &pxl_00, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_nw, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_se, &pxl_nw, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_nw, &pxl_00, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_se, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nw, &pxl_se, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_ne, &pxl_00, 1.0e-8));
    g_assert_true(satfire_pixels_are_adjacent(&pxl_00, &pxl_sw, 1.0e-8));
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ne, &pxl_sw, 1.0e-8));

    // should be false
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ww, &pxl_nn, 1.0e-8));

    g_assert_true(satfire_pixels_are_adjacent(&pxl_ss, &pxl_ee, 1.0e-8));

    // should be false
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ss, &pxl_ww, 1.0e-8));

    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_nn, 1.0e-8));

    // should be false
    g_assert_false(satfire_pixels_are_adjacent(&pxl_nn, &pxl_ww, 1.0e-8));

    g_assert_true(satfire_pixels_are_adjacent(&pxl_ee, &pxl_ss, 1.0e-8));

    // should be false
    g_assert_false(satfire_pixels_are_adjacent(&pxl_ww, &pxl_ss, 1.0e-8));

    g_assert_true(satfire_pixels_are_adjacent(&pxl_nn, &pxl_ee, 1.0e-8));

    // Checking that there is no overlap is not good enough since there may be some overlap due to
    // using the eps variable to make the matching fuzzy. We should also check to make sure that
    // any vertices that aren't close aren't contained inside the other pixel.

    // This pixel is inside pxl_00, but it shares a common lower right corner
    struct SFPixel sub_pxl_01 = {.ul = (struct SFCoord){.lat = 44.5, .lon = -119.5},
                                 .ll = (struct SFCoord){.lat = 44.0, .lon = -119.5},
                                 .lr = (struct SFCoord){.lat = 44.0, .lon = -119.0},
                                 .ur = (struct SFCoord){.lat = 44.5, .lon = -119.0}};

    g_assert_false(satfire_pixels_are_adjacent(&pxl_00, &sub_pxl_01, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&sub_pxl_01, &pxl_00, 1.0e-6));

    // This pixel overlaps pxl_00 and shares a right edge. These overlap, but aren't adjacent.
    struct SFPixel sub_pxl_02 = {.ul = (struct SFCoord){.lat = 45.0, .lon = -119.5},
                                 .ll = (struct SFCoord){.lat = 44.0, .lon = -119.5},
                                 .lr = (struct SFCoord){.lat = 44.0, .lon = -119.0},
                                 .ur = (struct SFCoord){.lat = 45.0, .lon = -119.0}};

    g_assert_false(satfire_pixels_are_adjacent(&pxl_00, &sub_pxl_02, 1.0e-6));
    g_assert_false(satfire_pixels_are_adjacent(&sub_pxl_02, &pxl_00, 1.0e-6));
}

// ------------------------------ Tests for PixelList Serialization -------------------------------
struct SFPixelListFixture {
    struct SFPixelList *list;
};

static void
satfire_pixel_list_test_setup(struct SFPixelListFixture fixture[static 1], gconstpointer _unused)
{
    struct SFPixel pixels[9] = {{.ul = (struct SFCoord){.lat = 46.0, .lon = -121.0},
                                 .ll = (struct SFCoord){.lat = 45.0, .lon = -121.0},
                                 .lr = (struct SFCoord){.lat = 45.0, .lon = -120.0},
                                 .ur = (struct SFCoord){.lat = 46.0, .lon = -120.0}},

                                {.ul = (struct SFCoord){.lat = 46.0, .lon = -120.0},
                                 .ll = (struct SFCoord){.lat = 45.0, .lon = -120.0},
                                 .lr = (struct SFCoord){.lat = 45.0, .lon = -119.0},
                                 .ur = (struct SFCoord){.lat = 46.0, .lon = -119.0}},

                                {.ul = (struct SFCoord){.lat = 46.0, .lon = -119.0},
                                 .ll = (struct SFCoord){.lat = 45.0, .lon = -119.0},
                                 .lr = (struct SFCoord){.lat = 45.0, .lon = -118.0},
                                 .ur = (struct SFCoord){.lat = 46.0, .lon = -118.0}},

                                {.ul = (struct SFCoord){.lat = 45.0000002, .lon = -121.0000002},
                                 .ll = (struct SFCoord){.lat = 44.0000002, .lon = -120.9999998},
                                 .lr = (struct SFCoord){.lat = 43.9999998, .lon = -120.0000002},
                                 .ur = (struct SFCoord){.lat = 44.9999998, .lon = -119.9999998}},

                                {.ul = (struct SFCoord){.lat = 45.0, .lon = -120.0},
                                 .ll = (struct SFCoord){.lat = 44.0, .lon = -120.0},
                                 .lr = (struct SFCoord){.lat = 44.0, .lon = -119.0},
                                 .ur = (struct SFCoord){.lat = 45.0, .lon = -119.0}},

                                {.ul = (struct SFCoord){.lat = 45.0, .lon = -119.0},
                                 .ll = (struct SFCoord){.lat = 44.0, .lon = -119.0},
                                 .lr = (struct SFCoord){.lat = 44.0, .lon = -118.0},
                                 .ur = (struct SFCoord){.lat = 45.0, .lon = -118.0}},

                                {.ul = (struct SFCoord){.lat = 44.0, .lon = -121.0},
                                 .ll = (struct SFCoord){.lat = 43.0, .lon = -121.0},
                                 .lr = (struct SFCoord){.lat = 43.0, .lon = -120.0},
                                 .ur = (struct SFCoord){.lat = 44.0, .lon = -120.0}},

                                {.ul = (struct SFCoord){.lat = 44.0, .lon = -120.0},
                                 .ll = (struct SFCoord){.lat = 43.0, .lon = -120.0},
                                 .lr = (struct SFCoord){.lat = 43.0, .lon = -119.0},
                                 .ur = (struct SFCoord){.lat = 44.0, .lon = -119.0}},

                                {.ul = (struct SFCoord){.lat = 44.0, .lon = -119.0},
                                 .ll = (struct SFCoord){.lat = 43.0, .lon = -119.0},
                                 .lr = (struct SFCoord){.lat = 43.0, .lon = -118.0},
                                 .ur = (struct SFCoord){.lat = 44.0, .lon = -118.0}}};

    fixture->list = satfire_pixel_list_new();

    for (unsigned int i = 0; i < sizeof(pixels) / sizeof(pixels[0]); ++i) {
        fixture->list = satfire_pixel_list_append(fixture->list, &pixels[i]);
    }
}

static void
satfire_pixel_list_test_teaddown(struct SFPixelListFixture fixture[static 1], gconstpointer _unused)
{
    fixture->list = satfire_pixel_list_destroy(fixture->list);
}

static void
satfire_pixel_list_test_binary_round_trip(struct SFPixelListFixture fixture[static 1],
                                          gconstpointer _unused)
{
    g_assert_cmpint(fixture->list->len, ==, 9);

    size_t buf_size = satfire_pixel_list_binary_serialize_buffer_size(fixture->list);

    unsigned char *buffer = 0;
    buffer = calloc(buf_size, sizeof(unsigned char));
    g_assert_true(buffer);

    size_t num_bytes_written = satfire_pixel_list_binary_serialize(fixture->list, buf_size, buffer);

    g_assert_cmpint(buf_size, ==, num_bytes_written);

    struct SFPixelList *decoded = satfire_pixel_list_binary_deserialize(buffer);

    g_assert_true(decoded);
    g_assert_cmpint(fixture->list->len, ==, decoded->len);

    for (unsigned int i = 0; i < decoded->len; ++i) {
        g_assert_true(
            satfire_pixels_approx_equal(&fixture->list->pixels[i], &decoded->pixels[i], DBL_MIN));
    }

    free(buffer);
    decoded = satfire_pixel_list_destroy(decoded);
}
*/
