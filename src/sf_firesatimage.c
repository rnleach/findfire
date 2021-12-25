#include <assert.h>
#include <libgen.h>
#include <netcdf.h>
#include <netcdf_mem.h>
#include <pthread.h>
#include <string.h>
#include <tgmath.h>
#include <zip.h>

#include "sf_private.h"
#include "sf_util.h"

/*
 * I'm not sure if libnetcdf is thread safe yet, however after a few quick internet searches it
 * appears that as recently as 2018, it was not. All access to libnetcdf needs to be protected
 * by a mutex.
 */
pthread_mutex_t netcdf_mtx = PTHREAD_MUTEX_INITIALIZER;

static bool
fire_sat_image_initialize_with_nc_handle(int file_id, struct SatFireImage *tgt)
{
    assert(tgt);

    bool success = false;

    tgt->nc_file_id = file_id;

    int xdimid = -1;
    int ydimid = -1;
    int status = nc_inq_dimid(file_id, "x", &xdimid);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving dimension id x: %s",
           nc_strerror(status));
    status = nc_inq_dimid(file_id, "y", &ydimid);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving dimension id y: %s",
           nc_strerror(status));

    status = nc_inq_dimlen(file_id, xdimid, &tgt->xlen);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving dimension size x: %s",
           nc_strerror(status));
    status = nc_inq_dimlen(file_id, ydimid, &tgt->ylen);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving dimension size y: %s",
           nc_strerror(status));

    int xvarid = -1;
    int yvarid = -1;
    status = nc_inq_varid(file_id, "x", &xvarid);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving x variable id: %s",
           nc_strerror(status));
    status = nc_inq_varid(file_id, "y", &yvarid);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving y variable id: %s",
           nc_strerror(status));

    status = nc_get_att_double(file_id, xvarid, "scale_factor", &tgt->trans.xscale);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving x scale factor: %s",
           nc_strerror(status));
    status = nc_get_att_double(file_id, yvarid, "scale_factor", &tgt->trans.yscale);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving y scale factor: %s",
           nc_strerror(status));
    status = nc_get_att_double(file_id, xvarid, "add_offset", &tgt->trans.xoffset);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving x offset: %s", nc_strerror(status));
    status = nc_get_att_double(file_id, yvarid, "add_offset", &tgt->trans.yoffset);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving y offset: %s", nc_strerror(status));

    int projection_var_id = -1;
    status = nc_inq_varid(file_id, "goes_imager_projection", &projection_var_id);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving projection variable: %s",
           nc_strerror(status));

    status = nc_get_att_double(file_id, projection_var_id, "semi_major_axis", &tgt->trans.req);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving semi_major_axis: %s",
           nc_strerror(status));
    status = nc_get_att_double(file_id, projection_var_id, "semi_minor_axis", &tgt->trans.rpol);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving semi_minor_axis: %s",
           nc_strerror(status));
    status =
        nc_get_att_double(file_id, projection_var_id, "perspective_point_height", &tgt->trans.H);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving perspective_point_height: %s",
           nc_strerror(status));
    tgt->trans.H += tgt->trans.req;
    status = nc_get_att_double(file_id, projection_var_id, "longitude_of_projection_origin",
                               &tgt->trans.lon0);
    Stopif(status != NC_NOERR, goto RETURN, "Error retrieving longitude_of_projection_origin: %s",
           nc_strerror(status));

    success = true;

RETURN:
    return success;
}

static bool
fire_sat_image_open_nc(char const *fname, struct SatFireImage *tgt)
{
    assert(fname);
    assert(tgt);

    bool success = false;

    int rc = pthread_mutex_lock(&netcdf_mtx);
    Stopif(rc, return false, "Error acquiring mutex.");

    int file_id = -1;
    int status = nc_open(fname, NC_NOWRITE, &file_id);
    Stopif(status != NC_NOERR, goto RETURN, "Error opening NetCDF %s: %s", fname,
           nc_strerror(status));

    success = fire_sat_image_initialize_with_nc_handle(file_id, tgt);

RETURN:
    rc = pthread_mutex_unlock(&netcdf_mtx);
    Stopif(rc, return false, "Error unlocking mutex.");

    return success;
}

static bool
fire_sat_image_open_zip(char const *fname, struct SatFireImage *tgt)
{
    assert(fname);
    assert(tgt);

    bool success = false;
    unsigned char *memory = 0;

    zip_t *zfile = zip_open(fname, ZIP_RDONLY, 0);
    Stopif(!zfile, goto RETURN, "Error opening file: %s", fname);

    assert(zip_get_num_entries(zfile, 0) == 1);

    struct zip_stat stat = {0};
    zip_stat_init(&stat);
    int rc = zip_stat_index(zfile, 0, 0, &stat);
    Stopif(rc, goto CLOSE_ZIP, "Error getting zip stat for %s", fname);
    Stopif(!(stat.valid && ZIP_STAT_SIZE), goto CLOSE_ZIP,
           "No file size returned from zip_stat for %s", fname);

    memory = malloc(stat.size + 10);
    Stopif(!memory, goto CLOSE_ZIP, "Out of memory.");
    tgt->memory = memory;
    tgt->memory_size = stat.size;

    zip_file_t *inner_zfile = zip_fopen_index(zfile, 0, 0);
    Stopif(!inner_zfile, goto CLEAN_UP_MEM, "Error opening file inside zip %s", fname);

    size_t num_bytes_read = zip_fread(inner_zfile, tgt->memory, tgt->memory_size);
    Stopif(num_bytes_read != tgt->memory_size, goto CLOSE_ZIP_INNER,
           "Error reading data from zip %s", fname);

    rc = pthread_mutex_lock(&netcdf_mtx);
    Stopif(rc, return false, "Error acquiring mutex.");

    int file_id = -1;
    int status = nc_open_mem(fname, NC_NOWRITE, tgt->memory_size, tgt->memory, &file_id);
    Stopif(status != NC_NOERR, goto RETURN, "Error opening NetCDF %s: %s", fname,
           nc_strerror(status));

    success = fire_sat_image_initialize_with_nc_handle(file_id, tgt);

    rc = pthread_mutex_unlock(&netcdf_mtx);

    if (rc) {
        fprintf(stderr, "Error unlocking mutex.\n");
        success = false;
    }

CLOSE_ZIP_INNER:
    zip_fclose(inner_zfile);

CLEAN_UP_MEM:
    if (!success) {
        free(memory);
        memset(tgt, 0, sizeof(*tgt));
    }

CLOSE_ZIP:
    zip_close(zfile);

RETURN:
    return success;
}

bool
fire_sat_image_open(char const *fname, struct SatFireImage *tgt)
{
    // Make sure the target is properly initialized
    memset(tgt, 0, sizeof(*tgt));

    bool success = false;

    char fname_copy[1024] = {0};
    int num_printed = snprintf(fname_copy, sizeof(fname_copy), "%s", fname);
    Stopif(num_printed >= sizeof(fname_copy), return false, "File name too long: %s", fname);

    char *bname = basename(fname_copy);
    num_printed = snprintf(tgt->fname, sizeof(tgt->fname), "%s", bname);
    Stopif(num_printed >= sizeof(tgt->fname), return false, "File name too long: %s", fname);

    if (strstr(fname, ".zip") != 0) {
        success = fire_sat_image_open_zip(fname, tgt);
    } else {
        success = fire_sat_image_open_nc(fname, tgt);
    }

    return success;
}

void
fire_sat_image_close(struct SatFireImage *dataset)
{
    int rc = pthread_mutex_lock(&netcdf_mtx);
    Stopif(rc, return, "Error acquiring mutex.");

    int status = nc_close(dataset->nc_file_id);
    Stopif(status != NC_NOERR, return, "Error closing NetCDF file %s: %s", dataset->fname,
           nc_strerror(status));

    rc = pthread_mutex_unlock(&netcdf_mtx);
    if (rc) {
        fprintf(stderr, "Error unlocking mutex.\n");
    }

    if (dataset->memory) {
        free(dataset->memory);
    }

    memset(dataset, 0, sizeof(*dataset));
}

struct XYCoord {
    double x;
    double y;
};

static inline struct XYCoord
satfire_convert_row_col_to_scan_angles(struct CoordTransform const *trans, double row, double col)
{
    double x = trans->xscale * col + trans->xoffset;
    double y = trans->yscale * row + trans->yoffset;

    return (struct XYCoord){.x = x, .y = y};
}

static inline struct SFCoord
satfire_convert_xy_to_latlon(struct CoordTransform const *trans, struct XYCoord xy)
{
    double sinx = sin(xy.x);
    double cosx = cos(xy.x);
    double siny = sin(xy.y);
    double cosy = cos(xy.y);
    double req = trans->req;
    double rpol = trans->rpol;
    double H = trans->H;
    double lon0 = trans->lon0;

    double a = sinx * sinx + cosx * cosx * (cosy * cosy + req * req / (rpol * rpol) * siny * siny);
    double b = -2.0 * H * cosx * cosy;
    double c = H * H - req * req;

    double rs = (-b - sqrt(b * b - 4.0 * a * c)) / (2.0 * a);

    double sx = rs * cosx * cosy;
    double sy = -rs * sinx;
    double sz = rs * cosx * siny;

    double lat =
        atan2(req * req * sz, rpol * rpol * sqrt((H - sx) * (H - sx) + sy * sy)) * 180.0 / M_PI;
    double lon = lon0 - atan2(sy, H - sx) * 180.0 / M_PI;

    return (struct SFCoord){.lat = lat, .lon = lon};
}

static inline double *
satfire_nc_extract_variable_double(struct SatFireImage const *fdata, char const *variable)
{
    double *vals = 0;

    int var_id = -1;
    int status = nc_inq_varid(fdata->nc_file_id, variable, &var_id);
    Stopif(status != NC_NOERR, goto ERR_RETURN, "Error reading %s variable id: %s", variable,
           nc_strerror(status));

    size_t vals_len = fdata->xlen * fdata->ylen;
    vals = malloc(vals_len * sizeof(double));
    assert(vals);

    size_t start[2] = {0, 0};
    size_t counts[2] = {fdata->ylen, fdata->xlen};
    status = nc_get_vara_double(fdata->nc_file_id, var_id, start, counts, vals);
    Stopif(status != NC_NOERR, goto ERR_RETURN, "Error reading %s variable values: %s", variable,
           nc_strerror(status));

    double scale_factor = 1.0;
    double add_offset = 0.0;
    double fill_value = 65535.0;

    status = nc_get_att_double(fdata->nc_file_id, var_id, "scale_factor", &scale_factor);
    Stopif(status != NC_NOERR && status != NC_ENOTATT, goto ERR_RETURN,
           "Error reading scale_factor attribute for %s: %s", variable, nc_strerror(status));
    bool skip_transform = status == NC_ENOTATT;
    status = nc_get_att_double(fdata->nc_file_id, var_id, "add_offset", &add_offset);
    Stopif(status != NC_NOERR && status != NC_ENOTATT, goto ERR_RETURN,
           "Error reading add_offset attribute for %s: %s", variable, nc_strerror(status));
    skip_transform = skip_transform && (status == NC_ENOTATT);

    status = nc_get_att_double(fdata->nc_file_id, var_id, "_FillValue", &fill_value);
    Stopif(status != NC_NOERR && status != NC_ENOTATT, goto ERR_RETURN,
           "Error reading _FillValue attribute for %s: %s", variable, nc_strerror(status));

    if (!skip_transform) {
        for (size_t i = 0; i < vals_len; ++i) {
            if (vals[i] != fill_value) {
                vals[i] = vals[i] * scale_factor + add_offset;
            } else {
                vals[i] = -INFINITY;
            }
        }
    } else {
        for (size_t i = 0; i < vals_len; ++i) {
            if (vals[i] == fill_value) {
                vals[i] = -INFINITY;
            }
        }
    }

    return vals;

ERR_RETURN:
    free(vals);
    return 0;
}

static inline signed short *
satfire_nc_extract_data_quality_flag(struct SatFireImage const *fdata)
{
    signed short *vals = 0;

    int var_id = -1;
    int status = nc_inq_varid(fdata->nc_file_id, "DQF", &var_id);
    Stopif(status != NC_NOERR, goto ERR_RETURN, "Error reading DQF variable id: %s",
           nc_strerror(status));

    size_t vals_len = fdata->xlen * fdata->ylen;
    vals = malloc(vals_len * sizeof(signed short));
    assert(vals);

    size_t start[2] = {0, 0};
    size_t counts[2] = {fdata->ylen, fdata->xlen};
    status = nc_get_vara_short(fdata->nc_file_id, var_id, start, counts, vals);
    Stopif(status != NC_NOERR, goto ERR_RETURN, "Error reading DQF variable values: %s",
           nc_strerror(status));

    return vals;

ERR_RETURN:
    free(vals);
    return 0;
}

static inline short *
satfire_nc_extract_mask(struct SatFireImage const *fdata)
{
    short *vals = 0;

    int var_id = -1;
    int status = nc_inq_varid(fdata->nc_file_id, "Mask", &var_id);
    Stopif(status != NC_NOERR, goto ERR_RETURN, "Error reading Mask variable id: %s",
           nc_strerror(status));

    size_t vals_len = fdata->xlen * fdata->ylen;
    vals = malloc(vals_len * sizeof(short));
    assert(vals);

    size_t start[2] = {0, 0};
    size_t counts[2] = {fdata->ylen, fdata->xlen};
    status = nc_get_vara_short(fdata->nc_file_id, var_id, start, counts, vals);
    Stopif(status != NC_NOERR, goto ERR_RETURN, "Error reading Mask variable values: %s",
           nc_strerror(status));

    return vals;

ERR_RETURN:
    free(vals);
    return 0;
}

GArray *
fire_sat_image_extract_fire_points(struct SatFireImage const *fdata)
{
    assert(fdata);

    GArray *points = 0;
    double *powers = 0;
    double *areas = 0;
    double *temperatures = 0;
    signed short *data_quality_flags = 0;
    short *masks = 0;
    points = g_array_new(false, true, sizeof(struct FirePoint));
    assert(points);

    int rc = pthread_mutex_lock(&netcdf_mtx);
    Stopif(rc, goto ERR_RETURN, "Error acquiring lock on libnetcdf.");

    powers = satfire_nc_extract_variable_double(fdata, "Power");
    Stopif(!powers, pthread_mutex_unlock(&netcdf_mtx); goto ERR_RETURN, "Error reading Power");

    areas = satfire_nc_extract_variable_double(fdata, "Area");
    Stopif(!areas, pthread_mutex_unlock(&netcdf_mtx); goto ERR_RETURN, "Error reading Area");

    temperatures = satfire_nc_extract_variable_double(fdata, "Temp");
    Stopif(!temperatures, pthread_mutex_unlock(&netcdf_mtx);
           goto ERR_RETURN, "Error reading Temperature");

    masks = satfire_nc_extract_mask(fdata);
    Stopif(!masks, pthread_mutex_unlock(&netcdf_mtx); goto ERR_RETURN, "Error reading Mask");

    data_quality_flags = satfire_nc_extract_data_quality_flag(fdata);
    Stopif(!data_quality_flags, pthread_mutex_unlock(&netcdf_mtx);
           goto ERR_RETURN, "Error reading Data Quality Flags");

    rc = pthread_mutex_unlock(&netcdf_mtx);
    Stopif(rc, goto ERR_RETURN, "Error releasing lock on libnetcdf.");

    for (int j = 0; j < fdata->ylen; ++j) {
        for (int i = 0; i < fdata->xlen; ++i) {

            double power_mw = powers[fdata->xlen * j + i];
            double area = areas[fdata->xlen * j + i];
            double temperature = temperatures[fdata->xlen * j + i];
            short mask = masks[fdata->xlen * j + i];
            signed short dqf = data_quality_flags[fdata->xlen * j + i];

            // 0 for a data quality flag indicates a good quality fire detection.
            if (dqf == 0) {

                double ips[5] = {i - 0.5, i - 0.5, i + 0.5, i + 0.5, i};
                double jps[5] = {j - 0.5, j + 0.5, j + 0.5, j - 0.5, j};

                struct XYCoord xys[5] = {0};
                struct SFCoord coords[5] = {0};

                for (size_t k = 0; k < sizeof(xys) / sizeof(xys[0]); ++k) {
                    xys[k] = satfire_convert_row_col_to_scan_angles(&fdata->trans, jps[k], ips[k]);
                    coords[k] = satfire_convert_xy_to_latlon(&fdata->trans, xys[k]);
                }

                double scan_angle = hypot(xys[4].x, xys[4].y) * 180.0 / M_PI;

                struct SFCoord ul = coords[0];
                struct SFCoord ll = coords[1];
                struct SFCoord lr = coords[2];
                struct SFCoord ur = coords[3];

                struct SFPixel pixel = {.ul = ul,
                                        .ll = ll,
                                        .lr = lr,
                                        .ur = ur,
                                        .power = power_mw,
                                        .area = area,
                                        .temperature = temperature,
                                        .mask_flag = mask,
                                        .data_quality_flag = dqf,
                                        .scan_angle = scan_angle};

                struct FirePoint pnt = {.x = i, .y = j, .pixel = pixel};
                points = g_array_append_val(points, pnt);
            }
        }
    }

    free(powers);
    free(areas);
    free(temperatures);
    free(data_quality_flags);
    free(masks);
    return points;

ERR_RETURN:

    if (points)
        g_array_unref(points);
    free(powers);
    free(areas);
    free(temperatures);
    free(data_quality_flags);
    free(masks);

    return 0;
}
