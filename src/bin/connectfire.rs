
fn main() {
    println!("Hello world.");
}


/*
/** \file connectfire.c
 * \brief Create several time series of fires by temporally connecting clusters (from findfire.c).
 *
 * Connect clusters from the output database of findfire to make time series of fires. Each time
 * series is given an ID and stored in a database with a start date and an end date. In the future
 * other statistics may be added to that database. Another table in the database will record the
 * relationship to clusters by associating a row number from the sqlite database with a fire ID
 * from the database table created by this program.
 */
// Standard C
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

// System installed libraries
#include <glib.h>

// My source libs
#include "kamel.h"

// My project headers
#include "satfire.h"
#include "sf_util.h"

/*-------------------------------------------------------------------------------------------------
 *                                        Global State
 *-----------------------------------------------------------------------------------------------*/
_Atomic(unsigned int) next_wildfire_id;

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
 *                                    Stats for this run.
 *-----------------------------------------------------------------------------------------------*/
static void
update_wildfire_stats(struct SFWildfireList const *curr, struct SFWildfire **longest,
                      struct SFWildfire **most_powerful, struct SFWildfire **hottest)
{
    assert(longest && most_powerful && hottest);

    // If the list is empty, just return.
    if (!curr) {
        return;
    }

    //
    // Get the maximums for the current list
    //
    struct SFWildfire const *longest_curr = satfire_wildfirelist_get(curr, 0);
    double longest_duration = satfire_wildfire_duration(longest_curr);

    struct SFWildfire const *most_powerful_curr = satfire_wildfirelist_get(curr, 0);
    double most_power = satfire_wildfire_max_power(most_powerful_curr);

    struct SFWildfire const *hottest_curr = satfire_wildfirelist_get(curr, 0);
    double hottest_temp = satfire_wildfire_max_temperature(hottest_curr);

    for (size_t i = 1; i < satfire_wildfirelist_len(curr); ++i) {
        struct SFWildfire const *tst = satfire_wildfirelist_get(curr, i);

        double duration = satfire_wildfire_duration(tst);
        double power = satfire_wildfire_max_power(tst);
        double temp = satfire_wildfire_max_temperature(tst);

        if (duration > longest_duration) {
            longest_curr = tst;
            longest_duration = duration;
        }

        if (power > most_power) {
            most_powerful_curr = tst;
            most_power = power;
        }

        if (temp > hottest_temp) {
            hottest_curr = tst;
            hottest_temp = temp;
        }
    }

    // Compare to the inputs
    bool update_longest = (*longest && longest_duration > satfire_wildfire_duration(*longest) &&
                           *longest != longest_curr) ||
                          (!*longest);
    if (update_longest) {
        satfire_wildfire_destroy(g_steal_pointer(longest));
        *longest = satfire_wildfire_clone(longest_curr);
    }

    bool update_power =
        (*most_powerful && most_power > satfire_wildfire_max_power(*most_powerful) &&
         *most_powerful != most_powerful_curr) ||
        (!*most_powerful);
    if (update_power) {
        satfire_wildfire_destroy(g_steal_pointer(most_powerful));
        *most_powerful = satfire_wildfire_clone(most_powerful_curr);
    }

    bool update_hottest = (*hottest && hottest_temp > satfire_wildfire_max_temperature(*hottest) &&
                           *hottest != hottest_curr) ||
                          (!*hottest);
    if (update_hottest) {
        satfire_wildfire_destroy(g_steal_pointer(hottest));
        *hottest = satfire_wildfire_clone(hottest_curr);
    }
}

/*-------------------------------------------------------------------------------------------------
 *                                   Output List of Fires as KML
 *-----------------------------------------------------------------------------------------------*/
static void
save_wildfire_list(struct SFWildfireList *fires, char const *fname)
{
    assert(fires);

    FILE *out = fopen(fname, "w");
    Stopif(!out, exit(EXIT_FAILURE), "error opening file: %s", fname);

    kamel_start_document(out);

    kamel_start_style(out, "fire");
    kamel_poly_style(out, "880000FF", true, false);
    kamel_icon_style(out, "http://maps.google.com/mapfiles/kml/shapes/firedept.png", 1.3);
    kamel_end_style(out);

    size_t len = satfire_wildfirelist_len(fires);
    for (size_t i = 0; i < len; ++i) {
        struct SFWildfire const *f = satfire_wildfirelist_get(fires, i);

        char id[32] = {0};
        snprintf(id, sizeof(id), "%u", satfire_wildfire_id(f));

        time_t starttm = satfire_wildfire_get_first_observed(f);
        struct tm start = *gmtime(&starttm);
        char start_buf[32] = {0};
        strftime(start_buf, sizeof(start_buf), "%Y-%m-%d %H:%M:%SZ", &start);

        time_t endtm = satfire_wildfire_get_last_observed(f);
        struct tm end = *gmtime(&endtm);
        char end_buf[32] = {0};
        strftime(end_buf, sizeof(end_buf), "%Y-%m-%d %H:%M:%SZ", &end);

        double duration = satfire_wildfire_duration(f);

        int days = (int)floor(duration / 60.0 / 60.0 / 24.0);
        double so_far = days * 60.0 * 60.0 * 24.0;

        int hours = (int)floor((duration - so_far) / 60.0 / 60.0);

        char desc[1024] = {0};
        snprintf(desc, sizeof(desc),
                 "ID: %u<br/>"
                 "Start: %s<br/>"
                 "End: %s<br/>"
                 "Duration: %d days %d hours<br/>"
                 "Max Power: %.0lf MW<br/>"
                 "Max Temperature: %.0lf Kelvin<br/>",
                 satfire_wildfire_id(f), start_buf, end_buf, days, hours,
                 satfire_wildfire_max_power(f), satfire_wildfire_max_temperature(f));

        kamel_start_folder(out, id, 0, false);

        kamel_start_placemark(out, id, desc, "#fire");
        struct SFCoord centroid = satfire_wildfire_centroid(f);
        kamel_point(out, centroid.lat, centroid.lon, 0.0);
        kamel_end_placemark(out);

        satfire_pixel_list_kml_write(out, satfire_wildfire_pixels(f));

        kamel_end_folder(out);
    }

    kamel_end_document(out);

    fclose(out);
}

/*-------------------------------------------------------------------------------------------------
 *                                   Processing For A Satellite
 *-----------------------------------------------------------------------------------------------*/
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

    struct SFWildfire *longest = 0;
    struct SFWildfire *most_powerful = 0;
    struct SFWildfire *hottest = 0;

    time_t current_time_step = 0;

    struct SFClusterRow *row = 0;
    size_t num_absorbed = 0;
    while ((row = satfire_cluster_db_query_rows_next(rows, row))) {

        time_t start = satfire_cluster_db_satfire_cluster_row_start(row);

        if (start != current_time_step) {

            size_t num_merged = satfire_wildfirelist_len(current_fires);
            old_fires = satfire_wildfirelist_merge_fires(current_fires, old_fires);
            num_merged -= satfire_wildfirelist_len(current_fires);

            size_t num_old = satfire_wildfirelist_len(current_fires);
            old_fires =
                satfire_wildfirelist_drain_stale_fires(current_fires, old_fires, current_time_step);
            num_old -= satfire_wildfirelist_len(current_fires);

            size_t num_new = satfire_wildfirelist_len(new_fires);
            current_fires = satfire_wildfirelist_extend(current_fires, new_fires);

            printf("Absorbed = %4ld Merged = %4ld Aged out = %4ld New = %4ld at %s", num_absorbed,
                   num_merged, num_old, num_new, ctime(&current_time_step));

            current_time_step = start;
            num_absorbed = 0;

            update_wildfire_stats(old_fires, &longest, &most_powerful, &hottest);

            // TODO: send old fires to a database thread.
        }

        bool cluster_merged = satfire_wildfirelist_update(current_fires, row);

        if (!cluster_merged) {

            unsigned int id = atomic_fetch_add(&next_wildfire_id, 1);
            new_fires = satfire_wildfirelist_create_add_fire(new_fires, id, row);

        } else {
            ++num_absorbed;
        }
    }

    old_fires = satfire_wildfirelist_merge_fires(current_fires, old_fires);
    old_fires = satfire_wildfirelist_extend(old_fires, current_fires);
    old_fires = satfire_wildfirelist_extend(old_fires, new_fires);

    update_wildfire_stats(old_fires, &longest, &most_powerful, &hottest);

    char fname[256] = {0};
    int num_printed = snprintf(fname, sizeof(fname), "%s_%s.kml", options.database_file,
                               satfire_satellite_name(sat));
    Stopif(num_printed >= sizeof(fname), exit(EXIT_FAILURE), "filename buffer overflow");
    save_wildfire_list(old_fires, fname);

    printf("\n\nRun Summary for satellite %s:\n\t"
           "    Old Fires: %5ld\n\t"
           "Current Fires: %5ld\n\t"
           "    New Fires: %5ld\n\n\n",
           satfire_satellite_name(sat), satfire_wildfirelist_len(old_fires),
           satfire_wildfirelist_len(current_fires), satfire_wildfirelist_len(new_fires));

    // TODO: send old fires to a database thread

    current_fires = satfire_wildfirelist_destroy(current_fires);
    new_fires = satfire_wildfirelist_destroy(new_fires);
    old_fires = satfire_wildfirelist_destroy(old_fires);

    int sc = satfire_cluster_db_query_rows_finalize(&rows);
    Stopif(sc, return, "Error finalizing row query, returning from function.");

    printf("\nLongest duration fire:\n");
    satfire_wildfire_print(longest);

    printf("\nMost powerul fire:\n");
    satfire_wildfire_print(most_powerful);

    printf("\nHottest fire:\n");
    satfire_wildfire_print(hottest);

    printf("\n\n");

    satfire_wildfire_destroy(longest);
    satfire_wildfire_destroy(most_powerful);
    satfire_wildfire_destroy(hottest);

    return;
}
/*-------------------------------------------------------------------------------------------------
 *                                             Main
 *-----------------------------------------------------------------------------------------------*/
int
main(int argc, char *argv[argc + 1])
{
    program_initialization(&argc, &argv);

    // Connect to the chosen database.
    SFDatabaseH db = satfire_db_connect(options.database_file);
    Stopif(!db, exit(EXIT_FAILURE), "Unable to connect to database %s", options.database_file);

    //
    // Restore state from where we last left off.
    //
    unsigned int next_id = satfire_fires_db_next_wildfire_id(db);

    if (options.verbose) {
        fprintf(stdout, "  Next wildfire ID number: %u\n", next_id);
    }

    atomic_init(&next_wildfire_id, next_id);

    time_t start = 0; // TODO: Figure this out from the database.
    time_t end = time(0);

    // Start out using the whole world! For this program, no reason to limit the area.
    struct SFBoundingBox area = {.ll = (struct SFCoord){.lat = -90.0, .lon = -180.0},
                                 .ur = (struct SFCoord){.lat = 90.0, .lon = 180.0}};

    //
    // Do the processing.
    //
    for (unsigned int sat = 0; sat < SATFIRE_SATELLITE_NUM; sat++) {
        process_rows_for_satellite(sat, start, end, area, db);
    }

    satfire_db_close(&db);
    program_finalization();

    return EXIT_SUCCESS;
}
*/
