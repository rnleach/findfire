#include "util.h"

#include <assert.h>
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

#include <ftw.h>

#include <glib.h>

#include "cpl_conv.h"
#include "cpl_error.h"
#include "cpl_string.h"
#include "gdal.h"

#include "firepoint.h"
#include "cluster.h"
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

static int
process_entry(const char *fpath, const struct stat *sb, int typeflag, struct FTW *ftwbuf)
{
    if (typeflag != FTW_F) {
        return 0;
    }

    if(strcmp(file_ext(fpath), "nc") != 0) {
        return 0;
    }

    return 0;
}

int
main()
{
    program_initialization();

    int err_code = nftw(data_dir, process_entry, 5, 0);
    Stopif(err_code, return EXIT_FAILURE, "Error walking directories.");

    struct FireSatImage fdata = {0};
    bool ok = fire_sat_image_open(fname, &fdata);
    Stopif(!ok, return EXIT_FAILURE, "Error opening %s", fname);

    GArray *points = fire_sat_image_extract_fire_points(&fdata);
    fire_sat_image_close(&fdata);
    
    GArray *clusters = clusters_from_fire_points(points);

    g_array_sort(clusters, cluster_desc_cmp);

    for(unsigned int i = 0; i < clusters->len; ++i) {

        struct Cluster *curr_clust = &g_array_index(clusters, struct Cluster, i);

        printf("Cluster: %2d, Lat: %10.6lf, Lon: %11.6lf, Count: %2d, Power: %5.0lfMW\n", 
                i, curr_clust->lat, curr_clust->lon, curr_clust->count, curr_clust->power);
    }

    g_array_unref(clusters);

    program_finalization();
}
