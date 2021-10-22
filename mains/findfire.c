#include "util.h"

#include <assert.h>
#include <limits.h>
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "cluster.h"
#include "database.h"
#include "firesatimage.h"

char const *database_file = "/home/ryan/wxdata/findfire.sqlite";
char const *data_dir = "/home/ryan/wxdata/GOESX";

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
    struct Cluster biggest_fire;
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
        .biggest_fire = {0},
        .biggest_sat = {0},
        .biggest_sector = {0},
        .biggest_start = 0,
        .biggest_end = 0,
        .num_clusters = 0,
        .num_power_lt_1mw = 0,
        .num_power_lt_10mw = 0,
    };
}

static struct ClusterStats
cluster_stats_update(struct ClusterStats stats, char const sat[static 4],
                     char const sector[static 5], time_t start, time_t end, struct Cluster *cluster)
{
    if (cluster->power > stats.biggest_fire.power) {
        stats.biggest_fire = *cluster;
        memcpy(stats.biggest_sat, sat, 3);
        memcpy(stats.biggest_sector, sector, 4);
        stats.biggest_start = start;
        stats.biggest_end = end;
    }

    if (cluster->power < 1.0) {
        stats.num_power_lt_1mw += 1;
    }

    if (cluster->power < 10.0) {
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

    printf("\nIndividual Cluster Stats\n\n"
           "Most Powerfull:\n"
           "     satellite: %s\n"
           "        sector: %s\n"
           "         start: %s"
           "           end: %s"
           "           Lat: %10.6lf\n"
           "           Lon: %11.6lf\n"
           "         Count: %2d\n"
           "        Radius: %06.3lf km\n"
           "         Power: %5.0lf MW\n\n"
           "        Counts:\n"
           "         Total: %10u\n"
           "  Power < 1 MW: %10u\n"
           "    Pct < 1 MW: %10u%%\n"
           " Power < 10 MW: %10u\n"
           "   Pct < 10 MW: %10u%%\n",
           stats.biggest_sat, stats.biggest_sector, start_str, end_str, stats.biggest_fire.lat,
           stats.biggest_fire.lon, stats.biggest_fire.count, stats.biggest_fire.radius,
           stats.biggest_fire.power, stats.num_clusters, stats.num_power_lt_1mw,
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

static struct ClusterListStats
cluster_list_stats_update(struct ClusterListStats clstats, struct ClusterList *clusters)
{
    unsigned int num_clust = cluster_list_length(clusters);

    if (num_clust > clstats.max_num_clusters) {
        clstats.max_num_clusters = num_clust;
        memcpy(clstats.max_num_clusters_sat, clusters->satellite, 3);
        memcpy(clstats.max_num_clusters_sector, clusters->sector, 4);
        clstats.max_num_clusters_start = clusters->start;
        clstats.max_num_clusters_end = clusters->end;
    }

    if (num_clust < clstats.min_num_clusters) {
        clstats.min_num_clusters = num_clust;
        memcpy(clstats.min_num_clusters_sat, clusters->satellite, 3);
        memcpy(clstats.min_num_clusters_sector, clusters->sector, 4);
        clstats.min_num_clusters_start = clusters->start;
        clstats.min_num_clusters_end = clusters->end;
    }

    double total_power = cluster_list_total_power(clusters);
    if (total_power > clstats.max_total_power) {
        clstats.max_total_power = total_power;
        memcpy(clstats.max_total_power_sat, clusters->satellite, 3);
        memcpy(clstats.max_total_power_sector, clusters->sector, 4);
        clstats.max_total_power_start = clusters->start;
        clstats.max_total_power_end = clusters->end;
    }

    if (total_power < clstats.min_total_power) {
        clstats.min_total_power = total_power;
        memcpy(clstats.min_total_power_sat, clusters->satellite, 3);
        memcpy(clstats.min_total_power_sector, clusters->sector, 4);
        clstats.min_total_power_start = clusters->start;
        clstats.min_total_power_end = clusters->end;
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

int
main()
{
    program_initialization();
    int rc = EXIT_FAILURE; // We'll set it to success once we've achieved succes.

    sqlite3 *cluster_db = 0;
    sqlite3_stmt *add_stmt = 0;

    cluster_db = cluster_db_connect(database_file);
    Stopif(!cluster_db, goto CLEANUP_AND_EXIT, "Error opening database.");
    add_stmt = cluster_db_prepare_to_add(cluster_db);
    Stopif(!add_stmt, goto CLEANUP_AND_EXIT, "Error preparing add statement.");

    // Stats on the most powerfull individual cluster.
    struct ClusterStats cluster_stats = cluster_stats_new();

    // Stats about the powerful satellite image, or one with the most...
    struct ClusterListStats clstats = cluster_list_stats_new();

    time_t newest_scan_start_time = cluster_db_newest_scan_start(cluster_db);

    struct DirWalkState dir_walk_state = dir_walk_new_with_root(data_dir);
    char const *path = dir_walk_next_path(&dir_walk_state);

    while (path) {

        if (!skip_path(path, newest_scan_start_time)) {

            printf("Processing: %s\n", path);

            struct ClusterList clusters = cluster_list_from_file(path);
            if (!clusters.error) {
                for (unsigned int i = 0; i < clusters.clusters->len; ++i) {

                    struct Cluster *curr_clust =
                        &g_array_index(clusters.clusters, struct Cluster, i);

                    int failure = cluster_db_add_row(add_stmt, clusters.satellite, clusters.sector,
                                                     clusters.start, curr_clust->lat,
                                                     curr_clust->lon, curr_clust->power,
                                                     curr_clust->radius, curr_clust->count);
                    Stopif(failure, goto CLEANUP_AND_EXIT, "Error adding row to database.");

                    cluster_stats =
                        cluster_stats_update(cluster_stats, clusters.satellite, clusters.sector,
                                             clusters.start, clusters.end, curr_clust);
                }

                clstats = cluster_list_stats_update(clstats, &clusters);

            } else {
                printf("    Error processing file.\n");
            }

            cluster_list_clear(&clusters);
        }

        path = dir_walk_next_path(&dir_walk_state);
    }

    dir_walk_destroy(&dir_walk_state);

    cluster_stats_print(cluster_stats);
    cluster_list_stats_print(clstats);

    rc = EXIT_SUCCESS;

CLEANUP_AND_EXIT:
    cluster_db_finalize_add(cluster_db, &add_stmt);
    cluster_db_close(&cluster_db);
    program_finalization();

    return rc;
}
