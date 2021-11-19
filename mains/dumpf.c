
/** \file dumpf.c
 * \brief Export clusters into a KML file.
 *
 * This program will export all the clusters in a requested region and time range into a KML file.
 */
// Standard C
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

// System installed libraries
#include <glib.h>

// My headers
#include "cluster.h"
#include "database.h"
#include "geo.h"
#include "satellite.h"
#include "util.h"

// Source Libraries
#include "kamel.h"

/*-------------------------------------------------------------------------------------------------
 *                          Program Initialization, Finalization, and Options
 *-----------------------------------------------------------------------------------------------*/
static struct DumpFOptions {
    char *database_file;
    char *kml_file;
    time_t start;
    time_t end;
    struct BoundingBox region;
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

    // Initialize with with environment variables and default values.
    if (getenv("CLUSTER_DB")) {
        asprintf(&options.database_file, "%s", getenv("CLUSTER_DB"));
        asprintf(&options.kml_file, "%s.kml", options.database_file);
    }

    options.verbose = false;

    // Parse command line options.
    GError *error = 0;
    GOptionContext *context = g_option_context_new("- Find clusters and add them to a database.");
    g_option_context_add_main_entries(context, option_entries, 0);
    g_option_context_parse(context, argc, argv, &error);
    Stopif(error, exit(EXIT_FAILURE), "Error parsing options: %s", error->message);

    Stopif(!options.database_file, exit(EXIT_FAILURE), "Invalid, database_file is NULL");

    // Print out options as configured.
    if (options.verbose) {
        fprintf(stdout, "  Database: %s\n", options.database_file);
        fprintf(stdout, "Output KML: %s\n", options.kml_file);
        // TODO print out start / start times and BoundingBox
        assert(false);
    }
}

static void
program_finalization()
{
    free(options.database_file);
    free(options.kml_file);
}

/*-------------------------------------------------------------------------------------------------
 *                             Save a Cluster in a KML File
 *-----------------------------------------------------------------------------------------------*/
void
save_cluster_kml(struct Cluster *biggest, time_t start, time_t end, enum Satellite sat,
                 enum Sector sector)
{
    // Return early if no output file is configured.
    if (!options.kml_file) {
        return;
    }

    FILE *out = fopen(options.kml_file, "wb");
    Stopif(!out, return, "Unable to open file for writing: %s", options.kml_file);

    kamel_start_document(out);

    kamel_start_style(out, "fire");
    kamel_poly_style(out, "880000FF", true, false);
    kamel_icon_style(out, "http://maps.google.com/mapfiles/kml/shapes/firedept.png", 1.3);
    kamel_end_style(out);

    kamel_start_folder(out, "BiggestFire", 0, true);
    kamel_timespan(out, start, end);

    char *description = 0;
    asprintf(&description, "Satellite: %s</br>Sector: %s</br>Power: %.0lf MW",
             satfire_satellite_name(sat), satfire_sector_name(sector),
             cluster_total_power(biggest));

    kamel_start_placemark(out, "Biggest Fire", description, "#fire");
    struct Coord centroid = pixel_list_centroid(cluster_pixels(biggest));
    kamel_point(out, centroid.lat, centroid.lon, 0.0);
    kamel_end_placemark(out);
    free(description);

    pixel_list_kml_write(out, cluster_pixels(biggest));

    kamel_end_folder(out);

    kamel_end_document(out);

    fclose(out);

    return;
}

/*-------------------------------------------------------------------------------------------------
 *                                             MAIN
 *-----------------------------------------------------------------------------------------------*/
int
main(int argc, char *argv[argc + 1])
{
    int rc = EXIT_FAILURE;
    program_initialization(&argc, &argv);

    printf("Hello world.\n");

    rc = EXIT_SUCCESS;

    program_finalization();

    return rc;
}
