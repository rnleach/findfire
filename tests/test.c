
#include <locale.h>
#include <glib.h>

#include "../src/geo.h"

static void
test_coord_are_close(void)
{
    struct Coord left = {.lat = 45.5, .lon = -120.0};
    g_assert_true(coord_are_close(left, left, 1.0e-8));
}

int
main(int argc, char *argv[static 1])
{
    setlocale(LC_ALL, "");

    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/geo/coordinates/coord_are_close", test_coord_are_close);

    return g_test_run();
}
