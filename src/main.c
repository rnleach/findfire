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
#include "firepoint.h"
#include "firesatimage.h"

char const *fname = "/home/ryan/wxdata/GOES/"
                    "OR_ABI-L2-FDCC-M6_G17_s20212050401167_e20212050403540_c20212050404121.nc";

char const *data_dir = "/home/ryan/wxdata/GOES/";

static void
program_initialization()
{
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

    DIR *dir = opendir(data_dir);
    Stopif(!dir, return EXIT_FAILURE, "Error opening data directory: %s", data_dir);

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

            char full_path[1024] = {0};

            strncat(full_path, data_dir, sizeof(full_path) - 1);
            int remaining = sizeof(full_path) - strnlen(full_path, sizeof(full_path));
            Stopif(remaining <= 0, return EXIT_FAILURE, "path buffer too small");

            strncat(full_path, "/", remaining - 1);
            remaining = sizeof(full_path) - strnlen(full_path, sizeof(full_path));
            Stopif(remaining <= 0, return EXIT_FAILURE, "path buffer too small");

            strncat(full_path, entry->d_name, remaining - 1);
            remaining = sizeof(full_path) - strnlen(full_path, sizeof(full_path));
            Stopif(remaining <= 0, return EXIT_FAILURE, "path buffer too small");

            printf("Processing: %s\n", entry->d_name);
            struct ClusterList clusters = cluster_list_from_file(full_path);
            if (!clusters.error) {
                for (unsigned int i = 0; i < clusters.clusters->len; ++i) {

                    struct Cluster *curr_clust =
                        &g_array_index(clusters.clusters, struct Cluster, i);

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

    printf("Lat: %10.6lf, Lon: %11.6lf, Count: %2d, Power: %5.0lfMW\n", biggest_fire.lat,
           biggest_fire.lon, biggest_fire.count, biggest_fire.power);

    program_finalization();
}
