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
#include "ogr_srs_api.h"


char const *fname =
    "NETCDF:\"/Volumes/MET2/wxdata/GOES/"
    "OR_ABI-L2-FDCC-M6_G17_s20212050401167_e20212050403540_c20212050404121.nc\":Power";


struct FirePoint {
    int x;
    int y;
    float lat;
    float lon;
    float power;
};


double 
great_circle_distance(struct FirePoint const a, struct FirePoint const b){
    double a_lat = a.lat * DEG2RAD;
    double b_lat = b.lat * DEG2RAD;
    double a_lon = a.lon * DEG2RAD;
    double b_lon = b.lon * DEG2RAD;

    double dlat2 = (b_lat - a_lat) / 2.0;
    double dlon2 = (b_lon - a_lon) / 2.0;

    double sin2_dlat = pow(sin(dlat2), 2.0);
    double sin2_dlon = pow(sin(dlon2), 2.0);

    double arc = 2.0 * asin(sqrt(sin2_dlat + sin2_dlon * cos(a_lat) * cos(b_lat)));

    return arc * 6371009.0;
}

struct cluster {
    double lat;
    double lon;
    double power;
    int count;
    bool valid;
};

int
cluster_cmp(const void *ap, const void *bp)
{
    struct cluster const *a = ap;
    struct cluster const *b = bp;

    if (!a->valid && !b->valid) return 0;
    if (!a->valid) return 1;
    if (!b->valid) return -1;
    if (a->power > b->power) return -1;
    if (a->power < b->power) return  1;
    return 0;
}

int
main()
{
    GDALDatasetH dataset = 0;
    GDALAllRegister();
    dataset = GDALOpen(fname, GA_ReadOnly);
    if(!dataset) {
        printf("Error opening %s\n", fname);
        return EXIT_FAILURE;
    }

    GDALDriverH driver = GDALGetDatasetDriver(dataset);
    printf("Driver: %s/%s\n\n", GDALGetDriverShortName(driver), GDALGetDriverLongName(driver));
    printf("Size is %dx%dx%d\n\n", GDALGetRasterXSize(dataset), GDALGetRasterYSize(dataset),
            GDALGetRasterCount(dataset));

    if( GDALGetProjectionRef(dataset)) {
        printf("Projection is '%s'\n\n", GDALGetProjectionRef(dataset));
    }

    double geo_trans[6] = {0};
    if(GDALGetGeoTransform(dataset, geo_trans) == CE_None){

        printf("Origin = (%.6f, %.6f)\n\n", geo_trans[0], geo_trans[3]);
        printf("Pixel Size = (%.6f, %.6f)\n\n", geo_trans[1], geo_trans[5]);
    }

    char **meta_data = GDALGetMetadata(dataset, 0);
    char  **curr = meta_data;
    while(*curr){
        printf("     %s\n", *curr);
        curr++;
    }
    char const *start_date_str = CSLFetchNameValue(meta_data, "NC_GLOBAL#time_coverage_start");
    char const *end_date_str = CSLFetchNameValue(meta_data, "NC_GLOBAL#time_coverage_end");
    assert(start_date_str && end_date_str);
    printf("The start time is: %s\nThe  end time is: %s\n\n", start_date_str, end_date_str);


    GDALRasterBandH band = GDALGetRasterBand(dataset, 1);
    int n_block_x_size = 0;
    int n_block_y_size = 0;
    GDALGetBlockSize(band, &n_block_x_size, &n_block_y_size);
    printf("Block = %dx%d Type = %s, ColorInterp = %s\n\n", n_block_x_size, n_block_y_size,
            GDALGetDataTypeName(GDALGetRasterDataType(band)),
            GDALGetColorInterpretationName(GDALGetRasterColorInterpretation(band)));

    int b_got_min = 0;
    int b_got_max = 0;
    double adf_min_max[2];
    adf_min_max[0] = GDALGetRasterMinimum(band, &b_got_min);
    adf_min_max[1] = GDALGetRasterMaximum(band, &b_got_max);
    if( !(b_got_min && b_got_max)) {
        GDALComputeRasterMinMax(band, true, adf_min_max);
    }
    printf("Min=%.3f, Max=%.3f\n\n", adf_min_max[0], adf_min_max[1]);

    if(GDALGetOverviewCount(band) > 0) {
        printf("Band has %d overviews.\n\n", GDALGetOverviewCount(band));
    }

    if(GDALGetRasterColorTable(band)) {
        printf("Band has a color table with %d entries.\n\n", 
                GDALGetColorEntryCount(GDALGetRasterColorTable(band)));
    }

    OGRSpatialReferenceH src_srs = GDALGetSpatialRef(dataset);
    assert(src_srs);

    OGRSpatialReferenceH dst_srs = OSRNewSpatialReference(0);
    //OSRSetWellKnownGeogCS(dst_srs, "WGS84");
    OSRImportFromEPSG(dst_srs, 4326);
    assert(dst_srs);

    OGRCoordinateTransformationH trans = OCTNewCoordinateTransformation(src_srs, dst_srs);
    assert(trans);

    int ysize = GDALGetRasterBandYSize(band);
    int xsize = GDALGetRasterBandXSize(band);

    GArray *buffer = g_array_sized_new(false, true, sizeof(float), xsize * ysize);
    buffer = g_array_set_size(buffer, xsize * ysize);

    CPLErr err = GDALRasterIO(band, GF_Read, 
            0, 0, xsize, ysize, 
            buffer->data, xsize, ysize, 
            GDT_Float32, 0, 0);

    if(err != CE_None){
        printf("Error reading raster data, aborting.");
        return EXIT_FAILURE;
    }

    GArray *points = g_array_new(false, true, sizeof(struct FirePoint));
    assert(points);

    for(int j = 0; j < ysize; ++j) {
        for(int i = 0; i < xsize; ++i){

            float power_mw = g_array_index(buffer, float, j * xsize + i);
            if (power_mw > 0.0) {

                double xp = geo_trans[0] + i * geo_trans[1] + j * geo_trans[2];
                double yp = geo_trans[3] + i * geo_trans[4] + j * geo_trans[5];
                double zp = 0.0;

                OCTTransform(trans, 1, &xp, &yp, &zp);

                //printf("(x,y) = (%4d, %4d), (x,y)=(%11.6f, %11.6f), power: %5.0f\n", 
                //        i, j, xp, yp, power_mw);

                struct FirePoint pnt = (struct FirePoint){
                    .x = i, .y = j, .lat = xp, .lon = yp, .power = power_mw
                };
                points = g_array_append_val(points, pnt);
            }
        }
    }

    g_array_unref(buffer);
    OCTDestroyCoordinateTransformation(trans);
    GDALClose(dataset);
    
    //printf("Found %d points with fire.\n", points->len);

    GArray *clusters = g_array_sized_new(false, true, sizeof(struct cluster), 100);
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

        struct cluster curr_clust = {0};
        curr_clust.lat = g_array_index(cluster_points, struct FirePoint, 0).lat;
        curr_clust.lon = g_array_index(cluster_points, struct FirePoint, 0).lon;
        curr_clust.power = g_array_index(cluster_points, struct FirePoint, 0).power;
        curr_clust.count = 1;
        curr_clust.valid = true;

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

    g_array_sort(clusters, cluster_cmp);

    for(unsigned int i = 0; i < clusters->len; ++i) {

        struct cluster *curr_clust = &g_array_index(clusters, struct cluster, i);

        printf("Cluster: %2d, Lat: %10.6lf, Lon: %11.6lf, Count: %2d, Power: %5.0lfMW\n", 
                i, curr_clust->lat, curr_clust->lon, curr_clust->count, curr_clust->power);
    }

    g_array_unref(clusters);
}
