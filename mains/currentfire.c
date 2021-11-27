/** \file current.c
 * \brief Export clusters from most recent image into a KML file.
 *
 * This program will export all the clusters from the latest image in the database for a given
 * satellite and sector as KML.
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
 *                                     Command Line Options
 *-----------------------------------------------------------------------------------------------*/
static struct CurrentFireOptions {
    char *database_file;
    char *kml_file;
    enum Satellite sat;
    enum Sector sector;
    bool verbose;

} options = {.sat = SATFIRE_SATELLITE_G17,
             .sector = SATFIRE_SECTOR_FULL,
             .verbose = false,
             .kml_file = 0,
             .database_file = 0};

static bool
parse_satellite(const char *arg_name, const char *arg_val, void *user_data, GError **error)
{
    assert(!user_data);

    enum Satellite sat = satfire_satellite_string_contains_satellite(arg_val);
    Stopif(sat == SATFIRE_SATELLITE_NONE, goto ERR_RETURN, "Invalid satellite");

    options.sat = sat;

    return true;

ERR_RETURN:
    g_set_error(error, G_OPTION_ERROR, G_OPTION_ERROR_FAILED,
                "Error parsing satellite arg: %s,"
                " it must be 'G16' or 'G17'",
                arg_val);

    return false;
}

static bool
parse_sector(const char *arg_name, const char *arg_val, void *user_data, GError **error)
{
    assert(!user_data);

    enum Sector sector = satfire_sector_string_contains_sector(arg_val);
    Stopif(sector == SATFIRE_SECTOR_NONE, goto ERR_RETURN, "Invalid sector");

    options.sector = sector;

    return true;

ERR_RETURN:
    g_set_error(error, G_OPTION_ERROR, G_OPTION_ERROR_FAILED,
                "Error parsing sector arg: %s,"
                " it must be 'FDCF', 'FDCC', 'FDCM1', or 'FDCM2'",
                arg_val);

    return false;
}

// clang-format off
static GOptionEntry option_entries[] = 
{
    {
        "output", 
        'o', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_FILENAME, 
        &options.kml_file, 
        "Output KML file path, FILE_NAME.", 
        "FILE_NAME"
    },
    {
        "satellite", 
        's', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_CALLBACK, 
        parse_satellite, 
        "The satellite SAT to select the latest data from. Must be 'G16' or 'G17'. Default 'G17'.", 
        "SAT"
    },
    {
        "sector", 
        'r', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_CALLBACK, 
        parse_sector, 
        "The satellite sector SECT to select the latest data from. Must be 'FDCF' (Full Disk), "
            "'FDCC' (CONUS), 'FDCM1' (Meso-sector 1), or 'FDCM2' (Meso-sector 2). Default 'FDCF'.", 
        "SECT"
    },
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

/*-------------------------------------------------------------------------------------------------
 *                              Program Initialization and Finalization
 *-----------------------------------------------------------------------------------------------*/
static void
program_initialization(int argc[static 1], char ***argv)
{
    // Force to use UTC timezone.
    setenv("TZ", "UTC", 1);
    tzset();

    // Parse command line options.
    GError *error = 0;
    GOptionContext *context = g_option_context_new("- Find clusters and add them to a database.");
    g_option_context_add_main_entries(context, option_entries, 0);
    g_option_context_parse(context, argc, argv, &error);
    Stopif(error, exit(EXIT_FAILURE), "Error parsing options: %s", error->message);
    g_option_context_free(context);

    // If options weren't set, back fill with default values.
    if (!options.database_file && getenv("CLUSTER_DB")) {
        asprintf(&options.database_file, "%s", getenv("CLUSTER_DB"));
    }

    if (!options.kml_file) {
        asprintf(&options.kml_file, "%s.kml", options.database_file);
    }

    Stopif(!options.database_file, exit(EXIT_FAILURE), "Invalid, database_file is NULL");
    Stopif(!options.kml_file, exit(EXIT_FAILURE), "Invalid, kml_file is NULL");
    Stopif(options.sat == SATFIRE_SATELLITE_NONE, exit(EXIT_FAILURE), "Invalid satellite");
    Stopif(options.sector == SATFIRE_SECTOR_NONE, exit(EXIT_FAILURE), "Invalid sector");

    // Print out options as configured.
    if (options.verbose) {
        fprintf(stdout, "\n\n");
        fprintf(stdout, "    Database: %s\n", options.database_file);
        fprintf(stdout, "  Output KML: %s\n", options.kml_file);
        fprintf(stdout, "   Satellite: %s\n", satfire_satellite_name(options.sat));
        fprintf(stdout, "      Sector: %s\n", satfire_sector_name(options.sector));
        fprintf(stdout, "\n\n");
    }
}

static void
program_finalization()
{
    // Free the memory allocated by the options.
    free(options.database_file);
    free(options.kml_file);
}

/*-------------------------------------------------------------------------------------------------
 *                                      Comparing Clusters
 *-----------------------------------------------------------------------------------------------*/
static int
cluster_row_descending_power_sort_order_compare(void const *a, void const *b)
{
    struct ClusterRow const *rowa = a;
    struct ClusterRow const *rowb = b;

    if (cluster_db_cluster_row_power(rowa) > cluster_db_cluster_row_power(rowb)) {
        return -1;
    } else if (cluster_db_cluster_row_power(rowa) < cluster_db_cluster_row_power(rowb)) {
        return 1;
    } else {
        return 0;
    }
}

/*-------------------------------------------------------------------------------------------------
 *                                             MAIN
 *-----------------------------------------------------------------------------------------------*/

static void
cluster_row_destructor_for_glib(void *p)
{
    struct ClusterRow *row = p;
    cluster_db_cluster_row_finalize(row);
}

int
main(int argc, char *argv[argc + 1])
{
    program_initialization(&argc, &argv);

    //
    // Get the time of the most recent scan matching the options
    //
    ClusterDatabaseH db = cluster_db_connect(options.database_file);
    Stopif(!db, exit(EXIT_FAILURE), "Unable to connect to database %s.", options.database_file);
    time_t latest = cluster_db_newest_scan_start(db, options.sat, options.sector);
    cluster_db_close(&db);
    Stopif(latest == 0, exit(EXIT_FAILURE),
           "No data in the database for satellite %s and sector %s.",
           satfire_satellite_name(options.sat), satfire_sector_name(options.sector));

    //
    // Query the database to get an iterator over the data and fill a sorted list.
    //

    // Default to cover the globe
    struct Coord ll = {.lat = -90.0, .lon = -180.0};
    struct Coord ur = {.lat = 90.0, .lon = 180.0};
    struct BoundingBox region = {.ll = ll, .ur = ur};

    ClusterDatabaseQueryRowsH rows = cluster_db_query_rows(
        options.database_file, options.sat, options.sector, latest, latest + 3600, region);
    struct ClusterRow *row = 0;
    GList *sorted_rows = 0;
    while ((row = cluster_db_query_rows_next(rows, 0))) {
        sorted_rows =
            g_list_insert_sorted(sorted_rows, row, cluster_row_descending_power_sort_order_compare);
    }

    cluster_db_query_rows_finalize(&rows);

    //
    // Output the list
    //
    FILE *out = fopen(options.kml_file, "w");
    Stopif(!out, exit(EXIT_FAILURE), "error opening file: %s", options.kml_file);

    kamel_start_document(out);

    kamel_start_style(out, "fire");
    kamel_icon_style(out, "http://maps.google.com/mapfiles/kml/shapes/firedept.png", 1.3);
    kamel_end_style(out);

    kamel_start_folder(out, satfire_satellite_name(options.sat), 0, false);

    unsigned int row_num = 0;
    for (GList *item = sorted_rows; item != 0; item = item->next, ++row_num) {

        struct ClusterRow *clust = item->data;

        char name_buf[16] = {0};
        char description_buf[128] = {0};

        int num_printed = snprintf(name_buf, sizeof(name_buf), "%u", row_num);
        if (num_printed >= sizeof(name_buf)) {
            name_buf[sizeof(name_buf) - 1] = '\0';
        }

        num_printed =
            snprintf(description_buf, sizeof(description_buf),
                     "<h3>Cluster Power: %.0lfMW</h3><h3>Max Scan Angle: %.0lf&deg;</h3>",
                     cluster_db_cluster_row_power(clust), cluster_db_cluster_row_scan_angle(clust));
        if (num_printed >= sizeof(description_buf)) {
            description_buf[sizeof(description_buf) - 1] = '\0';
        }

        kamel_start_folder(out, name_buf, 0, false);

        struct PixelList const *pixels = cluster_db_cluster_row_pixels(clust);
        struct Coord centroid = pixel_list_centroid(pixels);

        kamel_start_placemark(out, name_buf, description_buf, "#fire");
        kamel_point(out, centroid.lat, centroid.lon, 0.0);
        kamel_end_placemark(out);

        pixel_list_kml_write(out, pixels);

        kamel_end_folder(out);
    }

    kamel_end_folder(out);
    kamel_end_document(out);

    fclose(out);

    g_list_free_full(g_steal_pointer(&sorted_rows), cluster_row_destructor_for_glib);

    program_finalization();

    return EXIT_SUCCESS;
}