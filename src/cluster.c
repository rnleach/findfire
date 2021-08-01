#include "cluster.h"

#include <assert.h>
#include <stdbool.h>
#include <string.h>

#include "firepoint.h"
#include "firesatimage.h"
#include "util.h"

void
cluster_list_clear(struct ClusterList *tgt)
{
    if (tgt->clusters) {
        g_array_unref(tgt->clusters);
    }

    if (tgt->err_msg) {
        free(tgt->err_msg);
    }

    memset(tgt, 0, sizeof(struct ClusterList));
}

static char const *
find_satellite_start(char const *fname)
{
    char const *g17 = strstr(fname, "G17");
    if (g17) {
        return g17;
    }

    char const *g16 = strstr(fname, "G16");
    return g16;
}

static char const *
find_sector_start(char const *fname)
{
    char const *fdcc = strstr(fname, "FDCC");
    if (fdcc) {
        return fdcc;
    }

    char const *fdcf = strstr(fname, "FDCF");
    if (fdcf) {
        return fdcf;
    }

    char const *fdcm = strstr(fname, "FDCM");
    return fdcm;
}

static char const *
find_start_time(char const *fname)
{
    char const *start = strstr(fname, "_s");
    if(start) return start + 2;
    return start;
}

static char const *
find_end_time(char const *fname)
{
    char const *end = strstr(fname, "_e");
    if(end) return end + 2;
    return end;
}

struct ClusterList
cluster_list_from_file(char const *full_path)
{
    struct ClusterList clist = {0};
    char *err_msg = 0;
    GArray *points = 0;
    GArray *clusters = 0;

    char const *fname = get_file_name(full_path);

    // Get the satellite name
    char const *sat_start = find_satellite_start(fname);
    err_msg = "Error parsing satellite name";
    Stopif(!sat_start, goto ERR_RETURN, "Error parsing satellite name");
    memcpy(clist.satellite, sat_start, 3);

    // Get the sector name
    char const *sect_start = find_sector_start(fname);
    err_msg = "Error parsing sector name";
    Stopif(!sect_start, goto ERR_RETURN, "Error parsing sector name");
    memcpy(clist.sector, sect_start, 4);

    // Get the start and end times
    clist.start = parse_time_string(find_start_time(fname));
    clist.end = parse_time_string(find_end_time(fname));

    // Get the clusters member.
    struct FireSatImage fdata = {0};
    bool ok = fire_sat_image_open(full_path, &fdata);
    Stopif(!ok, err_msg = "Error opening NetCDF file";
           goto ERR_RETURN, "Error opening %s", full_path);

    points = fire_sat_image_extract_fire_points(&fdata);
    fire_sat_image_close(&fdata);

    clusters = clusters_from_fire_points(points);
    Stopif(!clusters, err_msg = "Error creating clusters.";
           goto ERR_RETURN, "Error creating clusters from fire points.");
    g_array_unref(points);

    clist.clusters = clusters;

    return clist;

ERR_RETURN:

    if (points) {
        g_array_unref(points);
        points = 0;
    }

    g_array_unref(clusters);
    clist.error = true;
    clist.err_msg = err_msg;
    return clist;
}

GArray *
clusters_from_fire_points(GArray const *points)
{
    GArray *clusters = g_array_sized_new(false, true, sizeof(struct Cluster), 100);
    GArray *cluster_points = g_array_sized_new(false, true, sizeof(struct FirePoint), 20);

    for (unsigned int i = 0; i < points->len; i++) {

        struct FirePoint *fp = &g_array_index(points, struct FirePoint, i);

        if (fp->x == 0 && fp->y == 0)
            continue;

        cluster_points = g_array_append_val(cluster_points, *fp);
        fp->x = 0;
        fp->y = 0;

        for (unsigned int j = i + 1; j < points->len; j++) {
            struct FirePoint *candidate = &g_array_index(points, struct FirePoint, j);

            if (candidate->x == 0 && candidate->y == 0)
                continue;
            for (unsigned int k = 0; k < cluster_points->len; ++k) {
                struct FirePoint *a_point_in_cluster =
                    &g_array_index(cluster_points, struct FirePoint, k);

                int dx = abs(a_point_in_cluster->x - candidate->x);
                int dy = abs(a_point_in_cluster->y - candidate->y);

                if (dx <= 1 && dy <= 1) {
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

        for (unsigned int j = 1; j < cluster_points->len; ++j) {

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

    return clusters;
}

int
cluster_desc_cmp(const void *ap, const void *bp)
{
    struct Cluster const *a = ap;
    struct Cluster const *b = bp;

    if (a->power > b->power)
        return -1;
    if (a->power < b->power)
        return 1;
    return 0;
}
