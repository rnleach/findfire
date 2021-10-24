#include <stdio.h>
#include <stdlib.h>
#include <time.h>

/**
 * \brief A wildfire.
 *
 * The id field identify the relationship between fires, if there is on. A new fire will be have
 * an id starting with a letter. If a wildfire has two or more clusters identified as belonging to
 * it at a later time, it will be a new wildfire with the same id as the parent with a number
 * appended. Then if there is another generation, a letter type will be appenended. And so on it
 * goes alternating between letters and numbers. A time series may go something like this:
 *
 * A
 * A
 * A1      A2
 * A1      A2
 * A1      A2A       A2B
 * A1      A2A       A2B
 * A1      A2A1 A2A2 A2B
 * A1A A1B A2A1 A2A2 A2B1 A2B2
 */
struct Wildfire {
    double lat;
    double lon;
    double radius;
    time_t last_observed;
    char id[32];
};

static void
program_initialization()
{
    // Force to use UTC timezone.
    setenv("TZ", "UTC", 1);
    tzset();
}

static void
program_finalization()
{
}

int
main()
{
    program_initialization();

    program_finalization();

    return EXIT_SUCCESS;
}
