#include "util.h"

#include "assert.h"
#include "math.h"
#include "stdbool.h"
#include "stdio.h"
#include "stdlib.h"

#include <glib.h>

#include "cpl_conv.h"
#include "cpl_error.h"
#include "cpl_string.h"
#include "gdal.h"

#include "firepoint.h"
#include "cluster.h"
#include "firesatimage.h"

char const *fname = "/Volumes/MET2/wxdata/GOES/"
    "OR_ABI-L2-FDCC-M6_G17_s20212050401167_e20212050403540_c20212050404121.nc";


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

    struct FireSatImage fdata = {0};
    bool ok = fire_sat_image_open(fname, &fdata);
    Stopif(!ok, return EXIT_FAILURE, "Error opening %s", fname);

    GArray *points = fire_sat_image_extract_fire_points(&fdata);
    fire_sat_image_close(&fdata);
    
    GArray *clusters = g_array_sized_new(false, true, sizeof(struct Cluster), 100);
    GArray *cluster_points = g_array_sized_new(false, true, sizeof(struct FirePoint), 20);

    for(unsigned int i = 0; i < points->len; i++){

        struct FirePoint *fp = &g_array_index(points, struct FirePoint, i);

        if(fp->x == 0 && fp->y == 0) continue;

        cluster_points = g_array_append_val(cluster_points, *fp);
        fp->x = 0;
        fp->y = 0;

        for(unsigned int j = i + 1; j < points->len; j++) {
            struct FirePoint *candidate = &g_array_index(points, struct FirePoint, j);

            if(candidate->x == 0 && candidate->y == 0) continue;
            for(unsigned int k = 0; k < cluster_points->len; ++k){
                struct FirePoint *a_point_in_cluster = &g_array_index(cluster_points, struct FirePoint, k);

                int dx = abs(a_point_in_cluster->x - candidate->x);
                int dy = abs(a_point_in_cluster->y - candidate->y);

                if(dx <= 1 && dy <= 1){
                    cluster_points = g_array_append_val(cluster_points, *candidate);
                    candidate->x = 0;
                    candidate->y = 0;
                }
            }
        }

        struct Cluster curr_clust = {0};
        curr_clust.lat = g_array_index(cluster_points, struct FirePoint, 0).lat;
        curr_clust.lon = g_array_index(cluster_points, struct FirePoint, 0).lon;
        curr_clust.power = g_array_index(cluster_points, struct FirePoint, 0).power;
        curr_clust.count = 1;

        for(unsigned int j = 1; j < cluster_points->len; ++j) {

            curr_clust.lat += g_array_index(cluster_points, struct FirePoint, j).lat;
            curr_clust.lon += g_array_index(cluster_points, struct FirePoint, j).lon;
            curr_clust.power += g_array_index(cluster_points, struct FirePoint, j).power;
            curr_clust.count += 1;
        }

        curr_clust.lat /= curr_clust.count;
        curr_clust.lon /= curr_clust.count;

        clusters = g_array_append_val(clusters, curr_clust);

        cluster_points = g_array_set_size(cluster_points, 0);
    }
    g_array_unref(cluster_points);

    g_array_sort(clusters, cluster_desc_cmp);

    for(unsigned int i = 0; i < clusters->len; ++i) {

        struct Cluster *curr_clust = &g_array_index(clusters, struct Cluster, i);

        printf("Cluster: %2d, Lat: %10.6lf, Lon: %11.6lf, Count: %2d, Power: %5.0lfMW\n", 
                i, curr_clust->lat, curr_clust->lon, curr_clust->count, curr_clust->power);
    }

    g_array_unref(clusters);

    program_finalization();
}
