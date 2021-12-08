#include <assert.h>
#include <libgen.h>
#include <tgmath.h>

#include <netcdf.h>

#include "sf_private.h"
#include "sf_util.h"

bool
fire_sat_image_open(char const *fname, struct SatFireImage *tgt)
{
    assert(fname);
    assert(tgt);

    char fname_copy[1024] = {0};
    int num_printed = snprintf(fname_copy, sizeof(fname_copy), "%s", fname);
    Stopif(num_printed >= sizeof(fname_copy), return false, "File name too long: %s", fname);

    char *bname = basename(fname_copy);
    num_printed = snprintf(tgt->fname, sizeof(tgt->fname), "%s", bname);
    Stopif(num_printed >= sizeof(tgt->fname), return false, "File name too long: %s", fname);

    int file_id = -1;
    int status = nc_open(fname, NC_NOWRITE, &file_id);
    Stopif(status != NC_NOERR, return false, "Error opening NetCDF %s: %s", fname,
           nc_strerror(status));
    tgt->nc_file_id = file_id;

    int xdimid = -1;
    int ydimid = -1;
    status = nc_inq_dimid(file_id, "x", &xdimid);
    Stopif(status != NC_NOERR, return false, "Error retrieving dimension id x: %s",
           nc_strerror(status));
    status = nc_inq_dimid(file_id, "y", &ydimid);
    Stopif(status != NC_NOERR, return false, "Error retrieving dimension id y: %s",
           nc_strerror(status));

    status = nc_inq_dimlen(file_id, xdimid, &tgt->xlen);
    Stopif(status != NC_NOERR, return false, "Error retrieving dimension size x: %s",
           nc_strerror(status));
    status = nc_inq_dimlen(file_id, ydimid, &tgt->ylen);
    Stopif(status != NC_NOERR, return false, "Error retrieving dimension size y: %s",
           nc_strerror(status));

    int xvarid = -1;
    int yvarid = -1;
    status = nc_inq_varid(file_id, "x", &xvarid);
    Stopif(status != NC_NOERR, return false, "Error retrieving x variable id: %s",
           nc_strerror(status));
    status = nc_inq_varid(file_id, "y", &yvarid);
    Stopif(status != NC_NOERR, return false, "Error retrieving y variable id: %s",
           nc_strerror(status));

    status = nc_get_att_double(file_id, xvarid, "scale_factor", &tgt->trans.xscale);
    Stopif(status != NC_NOERR, return false, "Error retrieving x scale factor: %s",
           nc_strerror(status));
    status = nc_get_att_double(file_id, yvarid, "scale_factor", &tgt->trans.yscale);
    Stopif(status != NC_NOERR, return false, "Error retrieving y scale factor: %s",
           nc_strerror(status));
    status = nc_get_att_double(file_id, xvarid, "add_offset", &tgt->trans.xoffset);
    Stopif(status != NC_NOERR, return false, "Error retrieving x offset: %s", nc_strerror(status));
    status = nc_get_att_double(file_id, yvarid, "add_offset", &tgt->trans.yoffset);
    Stopif(status != NC_NOERR, return false, "Error retrieving y offset: %s", nc_strerror(status));

    int projection_var_id = -1;
    status = nc_inq_varid(file_id, "goes_imager_projection", &projection_var_id);
    Stopif(status != NC_NOERR, return false, "Error retrieving projection variable: %s",
           nc_strerror(status));

    status = nc_get_att_double(file_id, projection_var_id, "semi_major_axis", &tgt->trans.req);
    Stopif(status != NC_NOERR, return false, "Error retrieving semi_major_axis: %s",
           nc_strerror(status));
    status = nc_get_att_double(file_id, projection_var_id, "semi_minor_axis", &tgt->trans.rpol);
    Stopif(status != NC_NOERR, return false, "Error retrieving semi_minor_axis: %s",
           nc_strerror(status));
    status =
        nc_get_att_double(file_id, projection_var_id, "perspective_point_height", &tgt->trans.H);
    Stopif(status != NC_NOERR, return false, "Error retrieving perspective_point_height: %s",
           nc_strerror(status));
    tgt->trans.H += tgt->trans.req;
    status = nc_get_att_double(file_id, projection_var_id, "longitude_of_projection_origin",
                               &tgt->trans.lon0);
    Stopif(status != NC_NOERR, return false, "Error retrieving longitude_of_projection_origin: %s",
           nc_strerror(status));

    return true;
}

void
fire_sat_image_close(struct SatFireImage *dataset)
{
    int status = nc_close(dataset->nc_file_id);
    Stopif(status != NC_NOERR, return, "Error closing NetCDF file %s: %s", dataset->fname,
           nc_strerror(status));
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

GArray *
fire_sat_image_extract_fire_points(struct SatFireImage const *fdata)
{
    assert(fdata);

    GArray *points = 0;
    double *powers = 0;
    points = g_array_new(false, true, sizeof(struct FirePoint));
    assert(points);

    int power_var_id = -1;
    int status = nc_inq_varid(fdata->nc_file_id, "Power", &power_var_id);
    Stopif(status != NC_NOERR, goto ERR_RETURN, "Error reading Power variable id: %s",
           nc_strerror(status));

    size_t power_len = fdata->xlen * fdata->ylen;
    powers = malloc(power_len * sizeof(double));
    assert(powers);

    size_t start[2] = {0, 0};
    size_t counts[2] = {fdata->ylen, fdata->xlen};
    status = nc_get_vara_double(fdata->nc_file_id, power_var_id, start, counts, powers);
    Stopif(status != NC_NOERR, goto ERR_RETURN, "Error reading Power variable values: %s",
           nc_strerror(status));

    for (int j = 0; j < fdata->ylen; ++j) {
        for (int i = 0; i < fdata->xlen; ++i) {

            double power_mw = powers[fdata->xlen * j + i];

            if (power_mw > 0.0) {

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
                                        .scan_angle = scan_angle};

                struct FirePoint pnt = {.x = i, .y = j, .pixel = pixel};
                points = g_array_append_val(points, pnt);
            }
        }
    }

    free(powers);
    return points;

ERR_RETURN:

    if (points)
        g_array_unref(points);
    free(powers);

    return 0;
}
