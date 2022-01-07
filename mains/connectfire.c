/** \file connectfire.c
 * \brief Create several time series of fires by temporally connecting clusters (from findfire.c).
 *
 * Connect clusters from the output database of findfire to make time series of fires. Each time
 * series is given an ID and stored in a database with a start date and an end date. In the future
 * other statistics may be added to that database. Another table in the database will record the
 * relationship to clusters by associating a row number from the sqlite database with a fire ID
 * from the database table created by this program.
 */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include <glib.h>

#include "satfire.h"

#include "sf_util.h"

#define DAYS_BACK 30
#define DAY_SEC (60 * 60 * 24)

/*-------------------------------------------------------------------------------------------------
 *                          Program Initialization, Finalization, and Options
 *-----------------------------------------------------------------------------------------------*/
static struct ConnectFireOptions {
    char *database_file;
    bool verbose;

} options = {0};

// clang-format off
static GOptionEntry option_entries[] = 
{
    {
        "verbose", 
        'v', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_NONE, 
        &options.verbose, 
        "Show verbose output.", 
        0
    },

    {NULL}
};
// clang-format on

static void
program_initialization(int argc[static 1], char ***argv)
{
    // Force to use UTC timezone.
    setenv("TZ", "UTC", 1);
    tzset();

    satfire_initialize();

    // Initialize with with environment variables and default values.
    if (getenv("CLUSTER_DB")) {
        asprintf(&options.database_file, "%s", getenv("CLUSTER_DB"));
    }

    // Parse command line options.
    GError *error = 0;
    GOptionContext *context = g_option_context_new("- Temporally connect clusters to form fires.");
    g_option_context_add_main_entries(context, option_entries, 0);
    g_option_context_parse(context, argc, argv, &error);
    Stopif(error, exit(EXIT_FAILURE), "Error parsing options: %s", error->message);
    g_option_context_free(context);

    Stopif(!options.database_file, exit(EXIT_FAILURE), "Invalid, database_file is NULL");

    // Print out options as configured.
    if (options.verbose) {
        fprintf(stdout, "  Database: %s\n", options.database_file);
    }

    satfire_db_initialize(options.database_file);
}

static void
program_finalization()
{
    free(options.database_file);

    satfire_finalize();
}

static void
process_rows_for_satellite(enum SFSatellite sat, time_t start, time_t end,
                           struct SFBoundingBox area, SFDatabaseH db)
{
    assert(db);

    SFClusterDatabaseQueryRowsH rows =
        satfire_cluster_db_query_rows(db, sat, SATFIRE_SECTOR_NONE, start, end, area);
    Stopif(!rows, return, "Error querying rows for %s, returning from function.",
           satfire_satellite_name(sat));

    struct SFWildfireList *current_fires = 0;
    struct SFWildfireList *new_fires = 0;
    struct SFWildfireList *old_fires = 0;

    time_t current_time_step = 0;

    struct SFClusterRow *row = 0;
    size_t num_merged = 0;
    while ((row = satfire_cluster_db_query_rows_next(rows, row))) {

        time_t start = satfire_cluster_db_satfire_cluster_row_start(row);

        if (start != current_time_step) {
            time_t oldest_allowed = current_time_step - DAYS_BACK * DAY_SEC;

            // moving on to a new time step, let's take some time to clean up.
            old_fires = satfire_wildfirelist_merge_fires(current_fires, old_fires);
            old_fires = satfire_wildfirelist_drain_fires_not_seen_since(current_fires, old_fires,
                                                                        oldest_allowed);

            // TODO: send old fires to a database thread.

            current_fires = satfire_wildfirelist_extend(current_fires, new_fires);

            current_time_step = start;

            printf("Merged = %ld\n\n", num_merged);
            num_merged = 0;
        }

        bool cluster_merged = satfire_wildfirelist_update(current_fires, row);

        if (!cluster_merged) {

            /*-----*/
            struct SFCoord centroid = satfire_cluster_db_satfire_cluster_row_centroid(row);

            printf("lat: %10.6lf lon: %11.6lf power: %6.0lf max_temperature: %4.0lf from %s %s %s",
                   centroid.lat, centroid.lon, satfire_cluster_db_satfire_cluster_row_power(row),
                   satfire_cluster_db_satfire_cluster_row_max_temperature(row),
                   satfire_satellite_name(satfire_cluster_db_satfire_cluster_row_satellite(row)),
                   satfire_sector_name(satfire_cluster_db_satfire_cluster_row_sector(row)),
                   ctime(&start));
            /*-----*/

            // TODO: Initialize the next id number from the database with a global, atomic constant.
            new_fires = satfire_wildfirelist_create_add_fire(new_fires, 0, row);

        } else {
            ++num_merged;
        }
    }

    old_fires = satfire_wildfirelist_merge_fires(current_fires, old_fires);

    // TODO: send old fires to a database thread
    // TODO: send current fires to a database thread
    // TODO: send new fires to a database thread

    current_fires = satfire_wildfirelist_destroy(current_fires);
    new_fires = satfire_wildfirelist_destroy(new_fires);
    old_fires = satfire_wildfirelist_destroy(old_fires);

    int sc = satfire_cluster_db_query_rows_finalize(&rows);

    Stopif(sc, return, "Error finalizing row query, returning from function.");

    return;
}
/*-------------------------------------------------------------------------------------------------
 *                                             Main
 *-----------------------------------------------------------------------------------------------*/
int
main(int argc, char *argv[argc + 1])
{
    program_initialization(&argc, &argv);

    time_t start = 0;
    time_t end = time(0);

    SFDatabaseH db = satfire_db_connect(options.database_file);
    struct SFBoundingBox area = {.ll = (struct SFCoord){.lat = -90.0, .lon = -180.0},
                                 .ur = (struct SFCoord){.lat = 90.0, .lon = 180.0}};

    for (unsigned int sat = 0; sat < SATFIRE_SATELLITE_NUM; sat++) {
        process_rows_for_satellite(sat, start, end, area, db);
    }

    satfire_db_close(&db);

    program_finalization();

    return EXIT_SUCCESS;
}
