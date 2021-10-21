#include "util.h"

#include <assert.h>
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
        // Skip full disk for now, I don't have many of those and they are much larger.
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

    struct Cluster biggest_fire = {0};
    char biggest_sat[4] = {0};
    char biggest_sector[5] = {0};
    time_t biggest_start = 0;
    time_t biggest_end = 0;

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

                    if (curr_clust->power > biggest_fire.power) {
                        biggest_fire = *curr_clust;
                        memcpy(biggest_sat, clusters.satellite, 3);
                        memcpy(biggest_sector, clusters.sector, 4);
                        biggest_start = clusters.start;
                        biggest_end = clusters.end;
                    }
                }
            } else {
                printf("    Error processing file.\n");
            }

            cluster_list_clear(&clusters);
        }

        path = dir_walk_next_path(&dir_walk_state);
    }

    dir_walk_destroy(&dir_walk_state);

    char start_str[128] = {0};
    ctime_r(&biggest_start, start_str);
    char end_str[128] = {0};
    ctime_r(&biggest_end, end_str);

    printf("\n\nCluster analysis metadata:\n"
           "     satellite: %s\n"
           "        sector: %s\n"
           "         start: %s"
           "           end: %s"
           "           Lat: %10.6lf\n"
           "           Lon: %11.6lf\n"
           "         Count: %2d\n"
           "        Radius: %06.3lfkm\n"
           "         Power: %5.0lfMW\n",
           biggest_sat, biggest_sector, start_str, end_str, biggest_fire.lat, biggest_fire.lon,
           biggest_fire.count, biggest_fire.radius, biggest_fire.power);

    rc = EXIT_SUCCESS;

CLEANUP_AND_EXIT:
    cluster_db_finalize_add(cluster_db, &add_stmt);
    cluster_db_close(&cluster_db);
    program_finalization();

    return rc;
}
