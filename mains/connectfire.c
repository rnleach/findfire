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

#include "satfire.h"

#include "sf_util.h"

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

/*-------------------------------------------------------------------------------------------------
 *                                             Main
 *-----------------------------------------------------------------------------------------------*/
int
main(int argc, char *argv[argc + 1])
{
    program_initialization(&argc, &argv);

    program_finalization();

    return EXIT_SUCCESS;
}
