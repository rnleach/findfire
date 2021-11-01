#include "cluster.h"

#include <assert.h>
#include <stdbool.h>
#include <string.h>

#include "firepoint.h"
#include "firesatimage.h"
#include "geo.h"
#include "util.h"

/*-------------------------------------------------------------------------------------------------
                                                 Cluster
-------------------------------------------------------------------------------------------------*/
struct Cluster {
    /// Average latitude of the points in the cluster.
    double lat;
    /// Average longitude of the points in the cluster.
    double lon;
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    double power;
    /// The distance from the cluster center to the farthest point in the cluster.
    double radius;
    /// The number of points that are in this cluster.
    int count;
};

struct Cluster *cluster_new(void)
{
    struct Cluster *new = malloc(sizeof(struct Cluster));
    Stopif(!new, exit(EXIT_FAILURE), "malloc fail: out of memory");

    *new = (struct Cluster){.lat=NAN, .lon=NAN, .power=0.0, .radius=0.0, .count=0};

    return new;
}

void cluster_destroy(struct Cluster **cluster)
{
    assert(cluster);
    assert(*cluster);
    free(*cluster);
    *cluster = 0;
}

struct Cluster *cluster_copy(struct Cluster *cluster)
{
    assert(cluster);

    struct Cluster *copy = cluster_new();
    *copy = *cluster;

    return copy;
}

double cluster_total_power(struct Cluster *cluster)
{
    assert(cluster);
    return cluster->power;
}

double cluster_radius(struct Cluster *cluster)
{
    assert(cluster);
    return cluster->radius;
}

int cluster_pixel_count(struct Cluster *cluster)
{
    assert(cluster);
    return cluster->count;
}

struct Coord cluster_centroid(struct Cluster *cluster)
{
    assert(cluster);
    return (struct Coord){.lat=cluster->lat, .lon=cluster->lon};
}


int
cluster_descending_power_compare(const void *ap, const void *bp)
{
    struct Cluster const *a = ap;
    struct Cluster const *b = bp;

    if (a->power > b->power)
        return -1;
    if (a->power < b->power)
        return 1;
    return 0;
}
/*-------------------------------------------------------------------------------------------------
                                               ClusterList
-------------------------------------------------------------------------------------------------*/
void
cluster_list_clear(struct ClusterList *tgt)
{
    if (tgt->clusters) {
        g_array_unref(tgt->clusters);
    }

    // These are static strings!
    // if (tgt->err_msg) {
    //    free(tgt->err_msg);
    //}

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

char const *
cluster_find_start_time(char const *fname)
{
    char const *start = strstr(fname, "_s");
    if (start)
        return start + 2;
    return start;
}

static char const *
find_end_time(char const *fname)
{
    char const *end = strstr(fname, "_e");
    if (end)
        return end + 2;
    return end;
}

static void
local_cluster_destroy(void *cluster)
{
    struct Cluster **clst = cluster;
    cluster_destroy(clst);
}

static GArray *
clusters_from_fire_points(GArray const *points)
{
    GArray *clusters = g_array_sized_new(false, true, sizeof(struct Cluster *), 100);
    g_array_set_clear_func(clusters, local_cluster_destroy);

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

        struct Cluster *curr_clust = cluster_new();
        curr_clust->lat = g_array_index(cluster_points, struct FirePoint, 0).lat;
        curr_clust->lon = g_array_index(cluster_points, struct FirePoint, 0).lon;
        curr_clust->power = g_array_index(cluster_points, struct FirePoint, 0).power;
        curr_clust->count = 1;

        for (unsigned int j = 1; j < cluster_points->len; ++j) {

            curr_clust->lat += g_array_index(cluster_points, struct FirePoint, j).lat;
            curr_clust->lon += g_array_index(cluster_points, struct FirePoint, j).lon;
            curr_clust->power += g_array_index(cluster_points, struct FirePoint, j).power;
            curr_clust->count += 1;
        }

        curr_clust->lat /= curr_clust->count;
        curr_clust->lon /= curr_clust->count;

        for (unsigned int j = 1; j < cluster_points->len; ++j) {
            double pnt_lat = g_array_index(cluster_points, struct FirePoint, j).lat;
            double pnt_lon = g_array_index(cluster_points, struct FirePoint, j).lon;

            double gs_distance =
                great_circle_distance(pnt_lat, pnt_lon, curr_clust->lat, curr_clust->lon);

            if (gs_distance > curr_clust->radius) {
                curr_clust->radius = gs_distance;
            }
        }

        clusters = g_array_append_val(clusters, curr_clust);

        cluster_points = g_array_set_size(cluster_points, 0);
    }
    g_array_unref(cluster_points);

    return clusters;
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
    clist.start = parse_time_string(cluster_find_start_time(fname));
    clist.end = parse_time_string(find_end_time(fname));

    // Get the clusters member.
    struct FireSatImage fdata = {0};
    bool ok = fire_sat_image_open(full_path, &fdata);
    Stopif(!ok, err_msg = "Error opening NetCDF file";
           goto ERR_RETURN, "Error opening NetCDF file %s", full_path);

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

    if (clusters) {
        g_array_unref(clusters);
        clusters = 0;
    }

    clist.error = true;
    clist.err_msg = err_msg;
    return clist;
}

unsigned int
cluster_list_length(struct ClusterList tgt[static 1])
{
    return tgt->clusters->len;
}

double
cluster_list_total_power(struct ClusterList tgt[static 1])
{
    double sum = 0.0;
    for (unsigned int i = 0; i < tgt->clusters->len; i++) {
        sum += g_array_index(tgt->clusters, struct Cluster *, i)->power;
    }

    return sum;
}
