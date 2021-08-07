#include "util.h"

#include <assert.h>
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include <dirent.h>
#include <sys/stat.h>

#include <glib.h>

#include "cpl_conv.h"
#include "cpl_error.h"
#include "cpl_string.h"
#include "gdal.h"

#include "cluster.h"
#include "database.h"
#include "firepoint.h"
#include "firesatimage.h"

char const *database_file = "/home/ryan/wxdata/findfire.sqlite";
char const *data_dir = "/home/ryan/wxdata/GOES/";

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

int
main()
{
    program_initialization();
    int rc = EXIT_FAILURE; // We'll set it to success once we've achieved succes.

    sqlite3 *cluster_db = 0;
    sqlite3_stmt *add_stmt = 0;

    DIR *dir = opendir(data_dir);
    Stopif(!dir, goto CLEANUP_AND_EXIT, "Error opening data directory: %s", data_dir);

    cluster_db = cluster_db_connect(database_file);
    Stopif(!cluster_db, goto CLEANUP_AND_EXIT, "Error opening database.");
    add_stmt = cluster_db_prepare_to_add(cluster_db);
    Stopif(!add_stmt, goto CLEANUP_AND_EXIT, "Error preparing add statement.");

    // Subtract 600 seconds to give 10 minutes of overlap.
    time_t newest_scan_start_time = cluster_db_newest_scan_start(cluster_db) - 600;

    struct Cluster biggest_fire = {0};
    char biggest_sat[4] = {0};
    char biggest_sector[5] = {0};
    time_t biggest_start = 0;
    time_t biggest_end = 0;

    struct dirent *entry = 0;
    while ((entry = readdir(dir))) {
        if (entry->d_type == DT_REG) {

            if (strcmp("nc", file_ext(entry->d_name)) != 0) {
                continue;
            }

            // Skip full disk for now, I don't have many of those and they are much larger.
            if (strstr(entry->d_name, "FDCF")) {
                continue;
            }
            // Skip meso-sector for now, I don't have many of those.
            if (strstr(entry->d_name, "FDCM")) {
                continue;
            }

            time_t scan_start = parse_time_string(cluster_find_start_time(entry->d_name));
            if (scan_start < newest_scan_start_time) {
                // Don't try to add data that's already there.
                continue;
            }

            char full_path[1024] = {0};

            strncat(full_path, data_dir, sizeof(full_path) - 1);
            int remaining = sizeof(full_path) - strnlen(full_path, sizeof(full_path));
            Stopif(remaining <= 0, goto CLEANUP_AND_EXIT, "path buffer too small");

            strncat(full_path, "/", remaining - 1);
            remaining = sizeof(full_path) - strnlen(full_path, sizeof(full_path));
            Stopif(remaining <= 0, goto CLEANUP_AND_EXIT, "path buffer too small");

            strncat(full_path, entry->d_name, remaining - 1);
            remaining = sizeof(full_path) - strnlen(full_path, sizeof(full_path));
            Stopif(remaining <= 0, goto CLEANUP_AND_EXIT, "path buffer too small");

            printf("Processing: %s\n", entry->d_name);
            struct ClusterList clusters = cluster_list_from_file(full_path);
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
    }

    char start_str[128] = {0};
    ctime_r(&biggest_start, start_str);
    char end_str[128] = {0};
    ctime_r(&biggest_end, end_str);

    printf("\n\nCluster analysis metadata:\n"
           "     satellite: %s\n"
           "        sector: %s\n"
           "         start: %s"
           "           end: %s",
           biggest_sat, biggest_sector, start_str, end_str);

    printf("Lat: %10.6lf, Lon: %11.6lf, Count: %2d, Radius: %06.3lfkm, Power: %5.0lfMW\n",
           biggest_fire.lat, biggest_fire.lon, biggest_fire.count, biggest_fire.radius,
           biggest_fire.power);

    rc = EXIT_SUCCESS;

CLEANUP_AND_EXIT:
    cluster_db_finalize_add(cluster_db, &add_stmt);
    cluster_db_close(&cluster_db);
    closedir(dir);
    program_finalization();

    return rc;
}
