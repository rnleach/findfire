/** \file connectfire.c
 * \brief Create several time series of fires by temporally connecting clusters (from findfire.c).
 *
 * Connect clusters from the output database of findfire to make time series of fires. Each time
 * series is given an ID and stored in a database with a start date and an end date. In the future
 * other statistics may be added to that database. Another table in the database will record the
 * relationship to clusters by associating a row number from the findfire.sqlite database with a
 * fire ID from the database created by this program.
 */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "satfire.h"

/**
 * \brief A wildfire.
 *
 * The id field identifies the relationship between clusters, if there is one. Each fire id will be
 * a 9 digit number followed by an underscore, then a 6 digit sub-fire number. Sub fires are
 * temporally contiguous, meaning they are matched in the next time step. If a time step comes up
 * where a sub fire disappears, then it's done, and if a cluster or hot spot appears there again it
 * will have a new sub-fire number. The fire (determined by the first 9 digits) may go undetected
 * for several days, even a month or more and still persist.
 *
 * e.g. id = 000001239_000056 is sub-fire number 56 of fire number 1239.
 *
 * \todo Still need implmented.
 */
struct Wildfire {
    time_t first_observed;
    time_t last_observed;
    unsigned int next_subfire;
    struct SFPixelList *area;
    char id[16];
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
