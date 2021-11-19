
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
 *                                     Command Line Options
 *-----------------------------------------------------------------------------------------------*/
static struct DumpFOptions {
    char *database_file;
    char *kml_file;
    time_t start;
    time_t end;
    struct BoundingBox region;
    bool verbose;

} options = {0};

static bool
parse_start_end(const char *arg_name, const char *arg_val, void *user_data, GError **error)
{
    assert(!user_data);

    // YYYY-MM-DD-HH
    Stopif(strlen(arg_val) < 13, goto ERR_RETURN, "Invalid date format.");

    int year = atoi(&arg_val[0]);
    int month = atoi(&arg_val[5]);
    int dom = atoi(&arg_val[8]);
    int hour = atoi(&arg_val[11]);

    Stopif(year <= 0 || month <= 0 || dom <= 0 || hour < 0, goto ERR_RETURN,
           "Invalid date format.");

    struct tm arg_time = {
        .tm_year = year - 1900, .tm_mon = month - 1, .tm_mday = dom, .tm_hour = hour};
    time_t arg_time_stamp = timegm(&arg_time);

    if (strcmp(arg_name, "-s") == 0 || strcmp(arg_name, "--start") == 0) {
        options.start = arg_time_stamp;
    } else if (strcmp(arg_name, "-e") == 0 || strcmp(arg_name, "--end") == 0) {
        options.end = arg_time_stamp;
    } else {
        goto ERR_RETURN;
    }

    return true;

ERR_RETURN:
    g_set_error(error, G_OPTION_ERROR, G_OPTION_ERROR_FAILED, "Error parsing time arg: %s",
                arg_val);

    return false;
}

static bool
parse_bounding_box(const char *arg_name, const char *arg_val, void *user_data, GError **error)
{
    assert(!user_data);

    char *old_c = (char *)arg_val;
    char *c = (char *)arg_val;
    double min_lat = strtod(c, &c);
    Stopif(min_lat == 0.0L && c == old_c, goto ERR_RETURN, "Error parsing minimum latitude from %s",
           arg_val);
    ++c; // move past the comma, worst case is the null character
    old_c = c;

    double min_lon = strtod(c, &c);
    Stopif(min_lon == 0.0L && c == old_c, goto ERR_RETURN,
           "Error parsing minimum longitude from %s", arg_val);
    ++c; // move past the comma, worst case is the null character
    old_c = c;

    double max_lat = strtod(c, &c);
    Stopif(max_lat == 0.0L && c == old_c, goto ERR_RETURN, "Error parsing maximum latitude from %s",
           arg_val);
    ++c; // move past the comma, worst case is the null character
    old_c = c;

    double max_lon = strtod(c, &c);
    Stopif(max_lon == 0.0L && c == old_c, goto ERR_RETURN,
           "Error parsing maximum longitude from %s", arg_val);

    struct Coord ll = {.lat = min_lat, .lon = min_lon};
    struct Coord ur = {.lat = max_lat, .lon = max_lon};
    options.region = (struct BoundingBox){.ll = ll, .ur = ur};

    return true;

ERR_RETURN:
    g_set_error(error, G_OPTION_ERROR, G_OPTION_ERROR_FAILED, "Error parsing bounding box arg: %s",
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
        "start", 
        's', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_CALLBACK, 
        parse_start_end, 
        "The start time in UTC, format YYYY-MM-DD-HH.", 
        "YYYY-MM-DD-HH"
    },
    {
        "end", 
        'e', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_CALLBACK, 
        parse_start_end, 
        "The end time in UTC, format YYYY-MM-DD-HH.", 
        "YYYY-MM-DD-HH"
    },
    {
        "region", 
        'r', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_CALLBACK, 
        parse_bounding_box, 
        "The region as a bounding box for which to extract data.", 
        "MIN_LAT,MIN_LON,MAX_LAT,MAX_LON"
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

    // If options weren't set, back fill with default values.
    if (!options.database_file && getenv("CLUSTER_DB")) {
        asprintf(&options.database_file, "%s", getenv("CLUSTER_DB"));
    }

    if (!options.kml_file) {
        asprintf(&options.kml_file, "%s.kml", options.database_file);
    }

    // Pick default start and end times to cover all time in the database.
    struct tm default_start = {.tm_year = 2000 - 1900, .tm_mon = 0, .tm_mday = 1, .tm_hour = 0};
    if (options.start == 0) {
        options.start = timegm(&default_start);
    }

    struct tm default_end = {.tm_year = 2050 - 1900, .tm_mon = 0, .tm_mday = 1, .tm_hour = 0};
    if (options.end == 0) {
        options.end = timegm(&default_end);
    }

    if (options.region.ll.lat == 0.0 && options.region.ll.lon == 0.0 &&
        options.region.ur.lat == 0.0 && options.region.ur.lon == 0.0) {

        // Default to cover all of Montana cause why not.
        struct Coord ll = {.lat = 44.0, .lon = -116.5};
        struct Coord ur = {.lat = 49.5, .lon = -104.0};

        options.region = (struct BoundingBox){.ll = ll, .ur = ur};
    }

    Stopif(!options.database_file, exit(EXIT_FAILURE), "Invalid, database_file is NULL");
    Stopif(!options.kml_file, exit(EXIT_FAILURE), "Invalid, kml_file is NULL");
    Stopif(options.start == 0, exit(EXIT_FAILURE), "Invalid start_time");
    Stopif(options.end == 0, exit(EXIT_FAILURE), "Invalid end_time");

    // Print out options as configured.
    if (options.verbose) {
        fprintf(stdout, "\n\n");
        fprintf(stdout, "    Database: %s\n", options.database_file);
        fprintf(stdout, "  Output KML: %s\n", options.kml_file);
        fprintf(stdout, "       Start: %s", ctime(&options.start));
        fprintf(stdout, "         End: %s", ctime(&options.end));
        fprintf(stdout, "Bounding Box: (%.6lf, %.6lf) <---> (%.6lf, %.6lf)\n",
                options.region.ll.lat, options.region.ll.lon, options.region.ur.lat,
                options.region.ur.lon);
        fprintf(stdout, "\n\n");
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
