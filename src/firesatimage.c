#include <assert.h>
#include <libgen.h>

#include "firesatimage.h"

#include "firepoint.h"
#include "util.h"

#include "ogr_srs_api.h"

bool
fire_sat_image_open(char const *fname, struct FireSatImage *tgt)
{
    assert(fname);
    assert(tgt);

    char fname_copy[1024] = {0};
    int num_printed = snprintf(fname_copy, sizeof(fname_copy), "%s", fname);
    Stopif(num_printed >= sizeof(fname_copy), return false, "File name too long: %s", fname);

    char *bname = basename(fname_copy);
    num_printed = snprintf(tgt->fname, sizeof(tgt->fname), "%s", bname);
    Stopif(num_printed >= sizeof(tgt->fname), return false, "File name too long: %s", fname);

    char descriptor[1024] = {0};
    num_printed = snprintf(descriptor, sizeof(descriptor), "NETCDF:\"%s\":Power", fname);
    Stopif(num_printed >= sizeof(descriptor), return false, "Descriptor buffer too small for %s",
           fname);

    tgt->dataset = GDALOpen(descriptor, GA_ReadOnly);
    Stopif(!tgt->dataset, return false, "Error opening %s", fname);

    CPLErr err = GDALGetGeoTransform(tgt->dataset, tgt->geo_transform);
    Stopif(err != CE_None, return false, "Error (%d) getting geo-transform from %s", err, fname);

    tgt->band = GDALGetRasterBand(tgt->dataset, 1);
    Stopif(!tgt->band, return false, "Error retrieving band-1 from the dataset in %s", fname);

    tgt->y_size = GDALGetRasterBandYSize(tgt->band);
    tgt->x_size = GDALGetRasterBandXSize(tgt->band);

    assert(tgt->x_size > 0);
    assert(tgt->y_size > 0);

    return true;
}

void
fire_sat_image_close(struct FireSatImage *dataset)
{
    memset(dataset->fname, 0, sizeof(dataset->fname));
    memset(dataset->geo_transform, 0, sizeof(dataset->geo_transform));
    dataset->x_size = 0;
    dataset->y_size = 0;
    GDALClose(dataset->dataset);
    dataset->dataset = 0;
}

GArray *
fire_sat_image_extract_fire_points(struct FireSatImage const *fdata)
{
    assert(fdata);

    GArray *buffer = 0;
    GArray *points = 0;
    OGRCoordinateTransformationH trans = 0;

    OGRSpatialReferenceH src_srs = GDALGetSpatialRef(fdata->dataset);
    assert(src_srs);

    OGRSpatialReferenceH dst_srs = OSRNewSpatialReference(0);
    OSRImportFromEPSG(dst_srs, 4326);
    assert(dst_srs);

    trans = OCTNewCoordinateTransformation(src_srs, dst_srs);
    assert(trans);
    OSRDestroySpatialReference(dst_srs);

    buffer = g_array_sized_new(false, true, sizeof(float), fdata->x_size * fdata->y_size);
    buffer = g_array_set_size(buffer, fdata->x_size * fdata->y_size);

    CPLErr err = GDALRasterIO(fdata->band, GF_Read, 0, 0, fdata->x_size, fdata->y_size,
                              buffer->data, fdata->x_size, fdata->y_size, GDT_Float32, 0, 0);

    Stopif(err != CE_None, goto ERR_RETURN, "Error reading raster data from %s", fdata->fname);

    points = g_array_new(false, true, sizeof(struct FirePoint));
    assert(points);

    for (int j = 0; j < fdata->y_size; ++j) {
        for (int i = 0; i < fdata->x_size; ++i) {

            float power_mw = g_array_index(buffer, float, j * fdata->x_size + i);
            if (power_mw > 0.0) {

                double xp = fdata->geo_transform[0] + i * fdata->geo_transform[1] +
                            j * fdata->geo_transform[2];
                double yp = fdata->geo_transform[3] + i * fdata->geo_transform[4] +
                            j * fdata->geo_transform[5];
                double zp = 0.0;

                OCTTransform(trans, 1, &xp, &yp, &zp);

                // printf("(x,y) = (%4d, %4d), (x,y)=(%11.6f, %11.6f), power: %5.0f\n",
                //        i, j, xp, yp, power_mw);

                struct FirePoint pnt =
                    (struct FirePoint){.x = i, .y = j, .lat = xp, .lon = yp, .power = power_mw};
                points = g_array_append_val(points, pnt);
            }
        }
    }

    g_array_unref(buffer);
    OCTDestroyCoordinateTransformation(trans);

    return points;

ERR_RETURN:

    if (buffer)
        g_array_unref(buffer);
    if (trans)
        OCTDestroyCoordinateTransformation(trans);
    if (points)
        g_array_unref(points);
    return 0;
}
