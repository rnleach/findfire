/** \file findfire.c
 * \brief Group individual satellite pixels showing wildfire into connected clusters.
 *
 * This program walks a directory tree and analyzes all the NOAA Big Data files with GOES satellite
 * Fire Detection Characteristics (FDC) data. Individual pixels with fire power greater than 0.0 MW
 * are grouped into clusters of adjacent pixels. The power is summed to get a total power for the
 * cluster, and then the total power and a geographic description of all the pixels in the cluster
 * are serialized and stored in a database. The source satellite, scanning sector (Full Disk, CONUS,
 * MesoSector), scan start, and scan end times are also stored in the database with each cluster.
 *
 * The goal of having all this data together is for other programs to read the data from the
 * database and perform more analysis.
 *
 * This program queries an existing database to find what the latest entry is in the database and
 * assumes it can skip any files that can contain older data.
 *
 * \todo: create more sophisticated checking for the oldest scan, or better yet for a scan existing
 * at all.
 *
 * At the end of processing, some summary statistics are printed to the screen and a file called
 * findfire.kml is output in the same location as the database file findfire.sqlite that has the
 * largest Cluster processed this time.
 */
#include "util.h"

#include <assert.h>
#include <limits.h>
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "cluster.h"
#include "courier.h"
#include "database.h"
#include "firesatimage.h"

char const *database_file = "/home/ryan/wxdata/findfire.sqlite";
char const *kml_file = "/home/ryan/wxdata/findfire.kml";
char const *data_dir = "/media/ryan/SAT/GOESX";

#if defined(__APPLE__) && defined(__MACH__)
#    define pthread_setname(a)
#elif defined(__linux__)
#    define pthread_setname(a) pthread_setname_np(pthread_self(), (a))
#endif

static void
program_initialization()
{
    // Force to use UTC timezone.
    setenv("TZ", "UTC", 1);
    tzset();

    GDALAllRegister();
}

static void
program_finalization()
{
}

static void
save_cluster_kml(struct Cluster *biggest, time_t start, time_t end, char const *sat,
                 char const *sector)
{
    FILE *out = fopen(kml_file, "wb");
    Stopif(!out, return, "Unable to open file for writing: %s", kml_file);

    kml_start_document(out);

    kml_start_style(out, "fire");
    kml_poly_style(out, "880000FF", true, false);
    kml_icon_style(out, "http://maps.google.com/mapfiles/kml/shapes/firedept.png", 1.3);
    kml_end_style(out);

    char *description = 0;
    asprintf(&description, "Satellite: %s</br>Sector: %s</br>Power: %.0lf", sat, sector,
             cluster_total_power(biggest));

    kml_start_placemark(out, "Biggest Fire", description, "#fire");
    kml_timespan(out, start, end);
    pixel_list_kml_write(out, cluster_pixels(biggest));
    kml_end_placemark(out);

    free(description);

    kml_end_document(out);

    fclose(out);

    return;
}

static bool
skip_path(char const *path, time_t newest_scan_start_time)
{
    if (strcmp("nc", file_ext(path)) != 0) {
        // Only process files with the '.nc' extension.
        return true;
    }

    if (strstr(path, "FDCF")) {
        // Skip Full Disk
        return true;
    }

    if (strstr(path, "FDCM")) {
        // Skip meso-sector for now, I don't have many of those.
        return true;
    }

    time_t scan_start = parse_time_string(cluster_find_start_time(path));
    if (scan_start < newest_scan_start_time) {
        // Don't try to add data that's already there.
        return true;
    }

    return false;
}

struct ClusterStats {
    struct Cluster *biggest_fire;
    char biggest_sat[4];
    char biggest_sector[5];
    time_t biggest_start;
    time_t biggest_end;

    unsigned num_clusters;
    unsigned num_power_lt_1mw;
    unsigned num_power_lt_10mw;
};

static struct ClusterStats
cluster_stats_new(void)
{
    return (struct ClusterStats){
        .biggest_fire = cluster_new(),
        .biggest_sat = {0},
        .biggest_sector = {0},
        .biggest_start = 0,
        .biggest_end = 0,
        .num_clusters = 0,
        .num_power_lt_1mw = 0,
        .num_power_lt_10mw = 0,
    };
}

static void
cluster_stats_destroy(struct ClusterStats *tgt)
{
    cluster_destroy(&tgt->biggest_fire);
    memset(tgt, 0, sizeof(struct ClusterStats));
}

static struct ClusterStats
cluster_stats_update(struct ClusterStats stats, char const sat[static 4],
                     char const sector[static 5], time_t start, time_t end, struct Cluster *cluster)
{
    double cluster_power = cluster_total_power(cluster);

    if (cluster_power > cluster_total_power(stats.biggest_fire)) {
        cluster_destroy(&stats.biggest_fire);
        stats.biggest_fire = cluster_copy(cluster);
        memcpy(stats.biggest_sat, sat, 3);
        memcpy(stats.biggest_sector, sector, 4);
        stats.biggest_start = start;
        stats.biggest_end = end;
    }

    if (cluster_power < 1.0) {
        stats.num_power_lt_1mw += 1;
    }

    if (cluster_power < 10.0) {
        stats.num_power_lt_10mw += 1;
    }

    stats.num_clusters += 1;

    return stats;
}

static void
cluster_stats_print(struct ClusterStats stats)
{
    char start_str[128] = {0};
    ctime_r(&stats.biggest_start, start_str);
    char end_str[128] = {0};
    ctime_r(&stats.biggest_end, end_str);

    struct Coord biggest_centroid = cluster_centroid(stats.biggest_fire);

    printf("\nIndividual Cluster Stats\n\n"
           "Most Powerfull:\n"
           "     satellite: %s\n"
           "        sector: %s\n"
           "         start: %s"
           "           end: %s"
           "           Lat: %10.6lf\n"
           "           Lon: %11.6lf\n"
           "         Count: %2d\n"
           "         Power: %5.0lf MW\n\n"
           "        Counts:\n"
           "         Total: %10u\n"
           "  Power < 1 MW: %10u\n"
           "    Pct < 1 MW: %10u%%\n"
           " Power < 10 MW: %10u\n"
           "   Pct < 10 MW: %10u%%\n",
           stats.biggest_sat, stats.biggest_sector, start_str, end_str, biggest_centroid.lat,
           biggest_centroid.lon, cluster_pixel_count(stats.biggest_fire),
           cluster_total_power(stats.biggest_fire), stats.num_clusters, stats.num_power_lt_1mw,
           stats.num_power_lt_1mw * 100 / stats.num_clusters, stats.num_power_lt_10mw,
           stats.num_power_lt_10mw * 100 / stats.num_clusters);
}

struct ClusterListStats {
    char min_num_clusters_sat[4];
    char min_num_clusters_sector[5];
    unsigned int min_num_clusters;
    time_t min_num_clusters_start;
    time_t min_num_clusters_end;

    char max_num_clusters_sat[4];
    char max_num_clusters_sector[5];
    unsigned int max_num_clusters;
    time_t max_num_clusters_start;
    time_t max_num_clusters_end;

    char max_total_power_sat[4];
    char max_total_power_sector[5];
    double max_total_power;
    time_t max_total_power_start;
    time_t max_total_power_end;

    char min_total_power_sat[4];
    char min_total_power_sector[5];
    double min_total_power;
    time_t min_total_power_start;
    time_t min_total_power_end;
};

static struct ClusterListStats
cluster_list_stats_new(void)
{
    return (struct ClusterListStats){
        .min_num_clusters_sat = {0},
        .min_num_clusters_sector = {0},
        .min_num_clusters = UINT_MAX,
        .min_num_clusters_start = 0,
        .min_num_clusters_end = 0,

        .max_num_clusters_sat = {0},
        .max_num_clusters_sector = {0},
        .max_num_clusters = 0,
        .max_num_clusters_start = 0,
        .max_num_clusters_end = 0,

        .max_total_power_sat = {0},
        .max_total_power_sector = {0},
        .max_total_power = 0.0,
        .max_total_power_start = 0,
        .max_total_power_end = 0,

        .min_total_power_sat = {0},
        .min_total_power_sector = {0},
        .min_total_power = HUGE_VAL,
        .min_total_power_start = 0,
        .min_total_power_end = 0,
    };
}

static void
cluster_list_stats_destroy(struct ClusterListStats *clstats)
{
    // Nothing to do at this time because nothing is heap allocated.
}

static struct ClusterListStats
cluster_list_stats_update(struct ClusterListStats clstats, struct ClusterList *clusters)
{
    unsigned int num_clust = cluster_list_length(clusters);

    if (num_clust > clstats.max_num_clusters) {
        clstats.max_num_clusters = num_clust;
        memcpy(clstats.max_num_clusters_sat, cluster_list_satellite(clusters), 3);
        memcpy(clstats.max_num_clusters_sector, cluster_list_sector(clusters), 4);
        clstats.max_num_clusters_start = cluster_list_scan_start(clusters);
        clstats.max_num_clusters_end = cluster_list_scan_end(clusters);
    }

    if (num_clust < clstats.min_num_clusters) {
        clstats.min_num_clusters = num_clust;
        memcpy(clstats.min_num_clusters_sat, cluster_list_satellite(clusters), 3);
        memcpy(clstats.min_num_clusters_sector, cluster_list_sector(clusters), 4);
        clstats.min_num_clusters_start = cluster_list_scan_start(clusters);
        clstats.min_num_clusters_end = cluster_list_scan_end(clusters);
    }

    double total_power = cluster_list_total_power(clusters);
    if (total_power > clstats.max_total_power) {
        clstats.max_total_power = total_power;
        memcpy(clstats.max_total_power_sat, cluster_list_satellite(clusters), 3);
        memcpy(clstats.max_total_power_sector, cluster_list_sector(clusters), 4);
        clstats.max_total_power_start = cluster_list_scan_start(clusters);
        clstats.max_total_power_end = cluster_list_scan_end(clusters);
    }

    if (total_power < clstats.min_total_power) {
        clstats.min_total_power = total_power;
        memcpy(clstats.min_total_power_sat, cluster_list_satellite(clusters), 3);
        memcpy(clstats.min_total_power_sector, cluster_list_sector(clusters), 4);
        clstats.min_total_power_start = cluster_list_scan_start(clusters);
        clstats.min_total_power_end = cluster_list_scan_end(clusters);
    }

    return clstats;
}

static void
cluster_list_stats_print(struct ClusterListStats clstats)
{
    char start_str[128] = {0};
    ctime_r(&clstats.max_total_power_start, start_str);
    char end_str[128] = {0};
    ctime_r(&clstats.max_total_power_end, end_str);

    printf("\n\n"
           "Max Image Power Stats:\n"
           "            satellite: %s\n"
           "               sector: %s\n"
           "                start: %s"
           "                  end: %s"
           "      Max Total Power: %.0lf GW\n\n",
           clstats.max_total_power_sat, clstats.max_total_power_sector, start_str, end_str,
           clstats.max_total_power / 100.0);

    ctime_r(&clstats.min_total_power_start, start_str);
    ctime_r(&clstats.min_total_power_end, end_str);

    printf("\n\n"
           "Min Image Power Stats:\n"
           "            satellite: %s\n"
           "               sector: %s\n"
           "                start: %s"
           "                  end: %s"
           "      Min Total Power: %.0lf MW\n\n",
           clstats.min_total_power_sat, clstats.min_total_power_sector, start_str, end_str,
           clstats.min_total_power);

    ctime_r(&clstats.max_num_clusters_start, start_str);
    ctime_r(&clstats.max_num_clusters_end, end_str);

    printf("\n\n"
           "Max Image Number Clusters:\n"
           "                satellite: %s\n"
           "                   sector: %s\n"
           "                    start: %s"
           "                      end: %s"
           "           Total Clusters: %u\n\n",
           clstats.max_num_clusters_sat, clstats.max_num_clusters_sector, start_str, end_str,
           clstats.max_num_clusters);

    ctime_r(&clstats.min_num_clusters_start, start_str);
    ctime_r(&clstats.min_num_clusters_end, end_str);

    printf("\n\n"
           "Min Image Number Clusters:\n"
           "                satellite: %s\n"
           "                   sector: %s\n"
           "                    start: %s"
           "                      end: %s"
           "           Total Clusters: %u\n\n",
           clstats.min_num_clusters_sat, clstats.min_num_clusters_sector, start_str, end_str,
           clstats.min_num_clusters);
}

/*-------------------------------------------------------------------------------------------------
 *                             Steps in the processing pipeline.
 *-----------------------------------------------------------------------------------------------*/
struct PipelineLink {
    Courier *from;
    Courier *to;
};

struct WalkerArgs {
    time_t newest_scan_start_time;
    Courier *to_cluster_list_loader;
};

static void *
directory_walker(void *arg)
{
    static char const threadname[] = "findfire-walker";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    struct WalkerArgs *args = arg;
    time_t newest_scan_start_time = args->newest_scan_start_time;
    Courier *to_cluster_list_loader = args->to_cluster_list_loader;

    courier_open(to_cluster_list_loader);

    struct DirWalkState dir_walk_state = dir_walk_new_with_root(data_dir);
    char const *path = dir_walk_next_path(&dir_walk_state);

    while (path) {
        if (!skip_path(path, newest_scan_start_time)) {
            printf("Processing: %s\n", path);

            char *owned_path = 0;
            asprintf(&owned_path, "%s", path);
            courier_send(to_cluster_list_loader, owned_path);
        }

        path = dir_walk_next_path(&dir_walk_state);
    }

    dir_walk_destroy(&dir_walk_state);

    courier_close(to_cluster_list_loader);
    return 0;
}

static void *
fire_cluster_list_loader(void *arg)
{
    static char const threadname[] = "findfire-loader";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    struct PipelineLink *links = arg;
    Courier *from_dir_walker = links->from;
    Courier *to_database = links->to;

    courier_open(to_database);
    courier_wait_until_ready(from_dir_walker);

    void *item = 0;
    while ((item = courier_receive(from_dir_walker))) {
        char *path = item;

        struct ClusterList *clusters = cluster_list_from_file(path);
        if (!cluster_list_error(clusters)) {
            courier_send(to_database, clusters);
        } else {
            printf("    Error processing file.\n");
            cluster_list_destroy(&clusters);
        }

        free(path);
    }

    courier_close(to_database);

    return 0;
}

static void *
database_filler(void *arg)
{
    static char const threadname[] = "findfire-dbase";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    sqlite3 *cluster_db = 0;
    sqlite3_stmt *add_stmt = 0;

    cluster_db = cluster_db_connect(database_file);
    Stopif(!cluster_db, goto CLEANUP_AND_RETURN, "Error opening database. (%s %u)", __FILE__,
           __LINE__);
    add_stmt = cluster_db_prepare_to_add(cluster_db);
    Stopif(!add_stmt, goto CLEANUP_AND_RETURN, "Error preparing add statement.");

    // Stats on individual clusters.
    struct ClusterStats cluster_stats = cluster_stats_new();

    // Stats about satellite images.
    struct ClusterListStats clstats = cluster_list_stats_new();

    Courier *from_cluster_list_loader = arg;
    courier_wait_until_ready(from_cluster_list_loader);

    void *item;
    while ((item = courier_receive(from_cluster_list_loader))) {
        struct ClusterList *clusters = item;

        GArray *clusters_array = cluster_list_clusters(clusters);

        const char *sat = cluster_list_satellite(clusters);
        const char *sector = cluster_list_sector(clusters);
        time_t start = cluster_list_scan_start(clusters);
        time_t end = cluster_list_scan_end(clusters);

        for (unsigned int i = 0; i < clusters_array->len; ++i) {

            struct Cluster *curr_clust = g_array_index(clusters_array, struct Cluster *, i);

            int failure = cluster_db_add_row(add_stmt, sat, sector, start, end, curr_clust);

            Stopif(failure, goto CLEANUP_AND_RETURN, "Error adding row to database.");

            cluster_stats =
                cluster_stats_update(cluster_stats, sat, sector, start, end, curr_clust);
        }

        clstats = cluster_list_stats_update(clstats, clusters);

        cluster_list_destroy(&clusters);
    }

    cluster_stats_print(cluster_stats);
    save_cluster_kml(cluster_stats.biggest_fire, cluster_stats.biggest_start,
                     cluster_stats.biggest_end, cluster_stats.biggest_sat,
                     cluster_stats.biggest_sector);
    cluster_stats_destroy(&cluster_stats);

    cluster_list_stats_print(clstats);
    cluster_list_stats_destroy(&clstats);

CLEANUP_AND_RETURN:
    cluster_db_finalize_add(cluster_db, &add_stmt);
    cluster_db_close(&cluster_db);
    return 0;
}

/*-------------------------------------------------------------------------------------------------
 *                                             MAIN
 *-----------------------------------------------------------------------------------------------*/
int
main()
{
    int rc = EXIT_FAILURE;
    program_initialization();

    Courier from_dir_walk = courier_new();
    Courier from_cluster_loader = courier_new();
    struct PipelineLink loader_link = {.from = &from_dir_walk, .to = &from_cluster_loader};

    pthread_t threads[4] = {0};
    sqlite3 *cluster_db = 0;

    cluster_db = cluster_db_connect(database_file);
    Stopif(!cluster_db, goto CLEANUP_AND_EXIT, "Error opening database. (%s %u)", __FILE__,
           __LINE__);

    time_t newest_scan_start_time = cluster_db_newest_scan_start(cluster_db);
    // Close it up and set it to NULL, we no longer need it and it will interefere with the other
    // threads if left open.
    cluster_db_close(&cluster_db);

    struct WalkerArgs walker_args = {.newest_scan_start_time = newest_scan_start_time,
                                     .to_cluster_list_loader = &from_dir_walk};

    int s = pthread_create(&threads[0], 0, directory_walker, &walker_args);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "directory_walker");
    s = pthread_create(&threads[1], 0, fire_cluster_list_loader, &loader_link);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "fire_cluster_list_loader");
    s = pthread_create(&threads[2], 0, fire_cluster_list_loader, &loader_link);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "fire_cluster_list_loader");
    s = pthread_create(&threads[3], 0, database_filler, &from_cluster_loader);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "database_filler");

    rc = EXIT_SUCCESS;

CLEANUP_AND_EXIT:

    // Already closed in successful case, but maybe not if there was an error. No harm in closing
    // it again since it will be NULL.
    cluster_db_close(&cluster_db);

    for (unsigned int i = 0; i < sizeof(threads) / sizeof(threads[0]); ++i) {
        if (threads[i]) {
            s = pthread_join(threads[i], 0);
            if (s) {
                fprintf(stderr, "Error joining thread %u\n", i);
                rc = EXIT_FAILURE;
            }
        }
    }

    courier_destroy(&from_cluster_loader);
    courier_destroy(&from_dir_walk);

    program_finalization();

    return rc;
}
