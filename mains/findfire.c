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
 * This program queries an existing database to find if a file has been processed already before
 * processing it.
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
#include "satellite.h"

char const *database_file = "/home/ryan/wxdata/findfire.sqlite";
char *kml_file = 0;
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

    // Initialize with with environment variables
    if (getenv("CLUSTER_DB")) {
        database_file = getenv("CLUSTER_DB");
    }
    asprintf(&kml_file, "%s.kml", database_file);

    if (getenv("SAT_ARCHIVE")) {
        data_dir = getenv("SAT_ARCHIVE");
    }

    fprintf(stdout, "Database: %s\n     KML: %s\n Archive: %s\n", database_file, kml_file,
            data_dir);
}

static void
program_finalization()
{
    free(kml_file);
}

static bool
skip_path(char const *path, ClusterDatabaseQueryPresentH query)
{
    if (strcmp("nc", file_ext(path)) != 0) {
        // Only process files with the '.nc' extension.
        return true;
    }

    enum Satellite satellite = satfire_satellite_string_contains_satellite(path);
    enum Sector sector = satfire_sector_string_contains_sector(path);

    if (satellite == SATFIRE_SATELLITE_NONE || sector == SATFIRE_SECTOR_NONE) {
        return true;
    }

    // Skip meso-sectors for now, I don't have many of those.
    if (sector == SATFIRE_SECTOR_MESO1 || sector == SATFIRE_SECTOR_MESO2) {
        return true;
    }

    // TODO: Need to add error checking to here. On error, parse_time_string should return the
    // time 0 since that is way out of bounds for the GOES R/S/T/.... era.
    time_t scan_start = parse_time_string(cluster_find_start_time(path));
    time_t scan_end = parse_time_string(cluster_find_end_time(path));

    int num_rows = cluster_db_present(query, satellite, sector, scan_start, scan_end);
    Stopif(num_rows < -1, return false, "Error querying num_rows, proceeding anyway.");

    if (num_rows >= 0) {
        return true;
    }

    return false;
}

static void
save_cluster_kml(struct Cluster *biggest, time_t start, time_t end, enum Satellite sat,
                 enum Sector sector)
{
    FILE *out = fopen(kml_file, "wb");
    Stopif(!out, return, "Unable to open file for writing: %s", kml_file);

    kml_start_document(out);

    kml_start_style(out, "fire");
    kml_poly_style(out, "880000FF", true, false);
    kml_icon_style(out, "http://maps.google.com/mapfiles/kml/shapes/firedept.png", 1.3);
    kml_end_style(out);

    char *description = 0;
    asprintf(&description, "Satellite: %s</br>Sector: %s</br>Power: %.0lf",
             satfire_satellite_name(sat), satfire_sector_name(sector),
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

struct ClusterStats {
    struct Cluster *biggest_fire;
    enum Satellite biggest_sat;
    enum Sector biggest_sector;
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
        .biggest_sat = SATFIRE_SATELLITE_NONE,
        .biggest_sector = SATFIRE_SECTOR_NONE,
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
cluster_stats_update(struct ClusterStats stats, enum Satellite sat, enum Sector sector,
                     time_t start, time_t end, struct Cluster *cluster)
{
    double cluster_power = cluster_total_power(cluster);

    if (cluster_power > cluster_total_power(stats.biggest_fire)) {
        cluster_destroy(&stats.biggest_fire);
        stats.biggest_fire = cluster_copy(cluster);
        stats.biggest_sat = sat;
        stats.biggest_sector = sector;
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
           satfire_satellite_name(stats.biggest_sat), satfire_sector_name(stats.biggest_sector),
           start_str, end_str, biggest_centroid.lat, biggest_centroid.lon,
           cluster_pixel_count(stats.biggest_fire), cluster_total_power(stats.biggest_fire),
           stats.num_clusters, stats.num_power_lt_1mw,
           stats.num_power_lt_1mw * 100 / stats.num_clusters, stats.num_power_lt_10mw,
           stats.num_power_lt_10mw * 100 / stats.num_clusters);
}

struct ClusterListStats {
    enum Satellite min_num_clusters_sat;
    enum Sector min_num_clusters_sector;
    unsigned int min_num_clusters;
    time_t min_num_clusters_start;
    time_t min_num_clusters_end;

    enum Satellite max_num_clusters_sat;
    enum Sector max_num_clusters_sector;
    unsigned int max_num_clusters;
    time_t max_num_clusters_start;
    time_t max_num_clusters_end;

    enum Satellite max_total_power_sat;
    enum Sector max_total_power_sector;
    double max_total_power;
    time_t max_total_power_start;
    time_t max_total_power_end;

    enum Satellite min_total_power_sat;
    enum Sector min_total_power_sector;
    double min_total_power;
    time_t min_total_power_start;
    time_t min_total_power_end;
};

static struct ClusterListStats
cluster_list_stats_new(void)
{
    return (struct ClusterListStats){
        .min_num_clusters_sat = SATFIRE_SATELLITE_NONE,
        .min_num_clusters_sector = SATFIRE_SECTOR_NONE,
        .min_num_clusters = UINT_MAX,
        .min_num_clusters_start = 0,
        .min_num_clusters_end = 0,

        .max_num_clusters_sat = SATFIRE_SATELLITE_NONE,
        .max_num_clusters_sector = SATFIRE_SECTOR_NONE,
        .max_num_clusters = 0,
        .max_num_clusters_start = 0,
        .max_num_clusters_end = 0,

        .max_total_power_sat = SATFIRE_SATELLITE_NONE,
        .max_total_power_sector = SATFIRE_SECTOR_NONE,
        .max_total_power = 0.0,
        .max_total_power_start = 0,
        .max_total_power_end = 0,

        .min_total_power_sat = SATFIRE_SATELLITE_NONE,
        .min_total_power_sector = SATFIRE_SECTOR_NONE,
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
        clstats.max_num_clusters_sat = cluster_list_satellite(clusters);
        clstats.max_num_clusters_sector = cluster_list_sector(clusters);
        clstats.max_num_clusters_start = cluster_list_scan_start(clusters);
        clstats.max_num_clusters_end = cluster_list_scan_end(clusters);
    }

    if (num_clust < clstats.min_num_clusters) {
        clstats.min_num_clusters = num_clust;
        clstats.min_num_clusters_sat = cluster_list_satellite(clusters);
        clstats.min_num_clusters_sector = cluster_list_sector(clusters);
        clstats.min_num_clusters_start = cluster_list_scan_start(clusters);
        clstats.min_num_clusters_end = cluster_list_scan_end(clusters);
    }

    double total_power = cluster_list_total_power(clusters);
    if (total_power > clstats.max_total_power) {
        clstats.max_total_power = total_power;
        clstats.max_total_power_sat = cluster_list_satellite(clusters);
        clstats.max_total_power_sector = cluster_list_sector(clusters);
        clstats.max_total_power_start = cluster_list_scan_start(clusters);
        clstats.max_total_power_end = cluster_list_scan_end(clusters);
    }

    if (total_power < clstats.min_total_power) {
        clstats.min_total_power = total_power;
        clstats.min_total_power_sat = cluster_list_satellite(clusters);
        clstats.min_total_power_sector = cluster_list_sector(clusters);
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
           satfire_satellite_name(clstats.max_total_power_sat),
           satfire_sector_name(clstats.max_total_power_sector), start_str, end_str,
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
           satfire_satellite_name(clstats.min_total_power_sat),
           satfire_sector_name(clstats.min_total_power_sector), start_str, end_str,
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
           satfire_satellite_name(clstats.max_num_clusters_sat),
           satfire_sector_name(clstats.max_num_clusters_sector), start_str, end_str,
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
           satfire_satellite_name(clstats.min_num_clusters_sat),
           satfire_sector_name(clstats.min_num_clusters_sector), start_str, end_str,
           clstats.min_num_clusters);
}

/*-------------------------------------------------------------------------------------------------
 *                             Steps in the processing pipeline.
 *-----------------------------------------------------------------------------------------------*/
struct PipelineLink {
    Courier *from;
    Courier *to;
};

static void *
directory_walker(void *arg)
{
    static char const threadname[] = "findfire-walker";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    Courier *to_cluster_list_loader = arg;

    ClusterDatabaseH cluster_db = 0;
    cluster_db = cluster_db_connect(database_file);
    Stopif(!cluster_db, exit(EXIT_FAILURE), "Error opening database. (%s %u)", __FILE__, __LINE__);

    ClusterDatabaseQueryPresentH present_query = 0;
    present_query = cluster_database_prepare_to_query_present(cluster_db);
    Stopif(!present_query, exit(EXIT_FAILURE), "Error preparing query. (%s %u)", __FILE__,
           __LINE__);

    courier_register_sender(to_cluster_list_loader);
    courier_wait_until_ready_to_send(to_cluster_list_loader);

    struct DirWalkState dir_walk_state = dir_walk_new_with_root(data_dir);
    char const *path = dir_walk_next_path(&dir_walk_state);

    while (path) {
        if (!skip_path(path, present_query)) {
            printf("Processing: %s\n", path);

            char *owned_path = 0;
            asprintf(&owned_path, "%s", path);
            bool success = courier_send(to_cluster_list_loader, owned_path);

            Stopif(!success, break, "Failed to send to loader.");
        }

        path = dir_walk_next_path(&dir_walk_state);
    }

    dir_walk_destroy(&dir_walk_state);
    courier_done_sending(to_cluster_list_loader);
    cluster_db_finalize_query_present(cluster_db, &present_query);
    cluster_db_close(&cluster_db);
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

    courier_register_receiver(from_dir_walker);
    courier_register_sender(to_database);

    courier_wait_until_ready_to_receive(from_dir_walker);
    courier_wait_until_ready_to_send(to_database);

    void *item = 0;
    while ((item = courier_receive(from_dir_walker))) {
        char *path = item;

        bool success_sending = true;

        struct ClusterList *clusters = cluster_list_from_file(path);
        if (!cluster_list_error(clusters)) {

            struct BoundingBox box = satfire_satellite_data_area(cluster_list_satellite(clusters));
            clusters = cluster_list_filter(clusters, box);

            success_sending = courier_send(to_database, clusters);
        } else {
            printf("    Error processing file.\n");
            cluster_list_destroy(&clusters);
        }

        free(path);

        Stopif(!success_sending, break, "Failed to send to database.");
    }

    courier_done_receiving(from_dir_walker);
    courier_done_sending(to_database);

    return 0;
}

static void *
database_filler(void *arg)
{
    static char const threadname[] = "findfire-dbase";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    Courier *from_cluster_list_loader = arg;
    courier_register_receiver(from_cluster_list_loader);
    courier_wait_until_ready_to_receive(from_cluster_list_loader);

    ClusterDatabaseH cluster_db = 0;
    ClusterDatabaseAddH add_stmt = 0;

    cluster_db = cluster_db_connect(database_file);
    Stopif(!cluster_db, goto CLEANUP_AND_RETURN, "Error opening database. (%s %u)", __FILE__,
           __LINE__);
    add_stmt = cluster_db_prepare_to_add(cluster_db);
    Stopif(!add_stmt, goto CLEANUP_AND_RETURN, "Error preparing add statement.");

    // Stats on individual clusters.
    struct ClusterStats cluster_stats = cluster_stats_new();

    // Stats about satellite images.
    struct ClusterListStats clstats = cluster_list_stats_new();

    void *item;
    while ((item = courier_receive(from_cluster_list_loader))) {
        struct ClusterList *clusters = item;

        int failure = cluster_db_add(cluster_db, add_stmt, clusters);
        Stopif(failure, goto CLEANUP_AND_RETURN, "Error adding row to database.");

        enum Satellite sat = cluster_list_satellite(clusters);
        enum Sector sector = cluster_list_sector(clusters);
        time_t start = cluster_list_scan_start(clusters);
        time_t end = cluster_list_scan_end(clusters);
        GArray *clusters_array = cluster_list_clusters(clusters);

        for (unsigned int i = 0; i < clusters_array->len; ++i) {

            struct Cluster *curr_clust = g_array_index(clusters_array, struct Cluster *, i);

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
    courier_done_receiving(from_cluster_list_loader);
    cluster_db_finalize_add(cluster_db, &add_stmt);
    cluster_db_close(&cluster_db);
    return 0;
}

/*-------------------------------------------------------------------------------------------------
 *                                             MAIN
 *-----------------------------------------------------------------------------------------------*/
static void
generic_destroy_cluster_list(void *cl)
{
    struct ClusterList *list = cl;
    cluster_list_destroy(&list);
}

int
main()
{
    int rc = EXIT_FAILURE;
    program_initialization();

    Courier from_dir_walk = courier_new();
    Courier from_cluster_loader = courier_new();
    struct PipelineLink loader_link = {.from = &from_dir_walk, .to = &from_cluster_loader};

    pthread_t threads[6] = {0};

    int s = pthread_create(&threads[0], 0, directory_walker, &from_dir_walk);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "directory_walker");
    s = pthread_create(&threads[1], 0, database_filler, &from_cluster_loader);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "database_filler");

    for (unsigned int i = 2; i < sizeof(threads) / sizeof(threads[0]); ++i) {
        s = pthread_create(&threads[i], 0, fire_cluster_list_loader, &loader_link);
        Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s(%u) thread.",
               "fire_cluster_list_loader", i - 2);
    }

    rc = EXIT_SUCCESS;

CLEANUP_AND_EXIT:

    for (unsigned int i = 0; i < sizeof(threads) / sizeof(threads[0]); ++i) {
        if (threads[i]) {
            s = pthread_join(threads[i], 0);
            if (s) {
                fprintf(stderr, "Error joining thread %u\n", i);
                rc = EXIT_FAILURE;
            }
        }
    }

    courier_destroy(&from_cluster_loader, generic_destroy_cluster_list);
    courier_destroy(&from_dir_walk, free);

    program_finalization();

    return rc;
}
