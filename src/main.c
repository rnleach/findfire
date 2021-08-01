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

    struct dirent *entry = 0;
    while((entry = readdir(dir))) {
        if (entry->d_type == DT_REG) {

            if(strcmp("nc", file_ext(entry->d_name)) != 0) {
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

        }
    }

    struct ClusterList clusters = cluster_list_from_file(fname);
    g_array_sort(clusters.clusters, cluster_desc_cmp);

    for (unsigned int i = 0; i < clusters.clusters->len; ++i) {

        struct Cluster *curr_clust = &g_array_index(clusters.clusters, struct Cluster, i);

        printf("Cluster: %2d, Lat: %10.6lf, Lon: %11.6lf, Count: %2d, Power: %5.0lfMW\n", i,
               curr_clust->lat, curr_clust->lon, curr_clust->count, curr_clust->power);
    }

    //g_array_unref(clusters);
    cluster_list_clear(&clusters);

    program_finalization();
}
