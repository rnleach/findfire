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
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    double power;
    /// Pixels making up the cluster.
    struct PixelList *pixels;
};

struct Cluster *
cluster_new(void)
{
    struct Cluster *new = malloc(sizeof(struct Cluster));
    Stopif(!new, exit(EXIT_FAILURE), "malloc fail: out of memory");

    *new = (struct Cluster){.power = 0.0, .pixels = pixel_list_new()};

    return new;
}

void
cluster_destroy(struct Cluster **cluster)
{
    assert(cluster);
    assert(*cluster);
    (*cluster)->pixels = pixel_list_destroy((*cluster)->pixels);
    free(*cluster);
    *cluster = 0;
}

void
cluster_add_fire_point(struct Cluster *cluster, struct FirePoint *fire_point)
{
    assert(cluster);
    assert(fire_point);

    cluster->pixels = pixel_list_append(cluster->pixels, &fire_point->pixel);
    cluster->power += fire_point->pixel.power;
}

struct Cluster *
cluster_copy(struct Cluster const *cluster)
{
    assert(cluster);

    struct Cluster *copy = malloc(sizeof(struct Cluster));
    Stopif(!copy, exit(EXIT_FAILURE), "malloc fail: out of memory");

    *copy = (struct Cluster){.power = cluster->power, .pixels = pixel_list_copy(cluster->pixels)};

    return copy;
}

double
cluster_total_power(struct Cluster const *cluster)
{
    assert(cluster);
    return cluster->power;
}

unsigned int
cluster_pixel_count(struct Cluster const *cluster)
{
    assert(cluster);

    return cluster->pixels->len;
}

const struct PixelList *
cluster_pixels(struct Cluster const *cluster)
{
    assert(cluster);
    return cluster->pixels;
}

struct Coord
cluster_centroid(struct Cluster const *cluster)
{
    assert(cluster);
    return pixel_list_centroid(cluster->pixels);
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
struct ClusterList {
    enum Sector sector;
    enum Satellite satellite;
    time_t start;     /**< Start time of the scan. */
    time_t end;       /**< End time of the scan*/
    GArray *clusters; /**< List of struct Cluster objects associated with the above metadata. */
    char *err_msg;    /**< Error message. */
    bool error;       /**< Error flag. False indicates no error. */
};

void
cluster_list_destroy(struct ClusterList **list)
{
    assert(list);
    assert(*list);

    if ((*list)->clusters) {
        g_array_unref((*list)->clusters);
    }

    // These are static strings!
    // if ((*list)->err_msg) {
    //    free((*list)->err_msg);
    //}

    free(*list);
    *list = 0;
}

enum Sector
cluster_list_sector(struct ClusterList *list)
{
    assert(list);
    return list->sector;
}

enum Satellite
cluster_list_satellite(struct ClusterList *list)
{
    assert(list);
    return list->satellite;
}

time_t
cluster_list_scan_start(struct ClusterList *list)
{
    assert(list);
    return list->start;
}

time_t
cluster_list_scan_end(struct ClusterList *list)
{
    assert(list);
    return list->end;
}

bool
cluster_list_error(struct ClusterList *list)
{
    assert(list);
    return list->error;
}

const char *
cluster_list_error_msg(struct ClusterList *list)
{
    assert(list);
    return list->err_msg;
}

GArray *
cluster_list_clusters(struct ClusterList *list)
{
    assert(list);

    if (list->error) {
        assert(!list->clusters); // Force to be NULL
    }

    return list->clusters;
}

struct ClusterList *
cluster_list_filter(struct ClusterList *list, struct BoundingBox box)
{
    assert(list);

    GArray *clusters = list->clusters;

    for (unsigned int i = 0; i < clusters->len; ++i) {
        struct Cluster *clust = g_array_index(clusters, struct Cluster *, i);

        struct Coord centroid = cluster_centroid(clust);

        if (!bounding_box_contains_coord(box, centroid, 0.0)) {
            clusters = g_array_remove_index_fast(clusters, i);
            --i; // Decrement this so we inspect this index again since a new value is there.
        }
    }

    list->clusters = clusters; // In case g_array_remove_index_fast() moved the array.

    return list;
}

char const *
cluster_find_start_time(char const *fname)
{
    char const *start = strstr(fname, "_s");
    if (start)
        return start + 2;
    return start;
}

char const *
cluster_find_end_time(char const *fname)
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
        struct FirePoint *curr_fire_point = &g_array_index(cluster_points, struct FirePoint, 0);
        cluster_add_fire_point(curr_clust, curr_fire_point);

        for (unsigned int j = 1; j < cluster_points->len; ++j) {

            curr_fire_point = &g_array_index(cluster_points, struct FirePoint, j);
            cluster_add_fire_point(curr_clust, curr_fire_point);
        }

        clusters = g_array_append_val(clusters, curr_clust);
        cluster_points = g_array_set_size(cluster_points, 0);
    }

    g_array_unref(cluster_points);

    return clusters;
}

struct ClusterList *
cluster_list_from_file(char const *full_path)
{
    struct ClusterList *clist = calloc(1, sizeof(struct ClusterList));
    char *err_msg = 0;
    GArray *points = 0;
    GArray *clusters = 0;

    char const *fname = get_file_name(full_path);

    // Get the satellite
    enum Satellite satellite = satfire_satellite_string_contains_satellite(fname);
    err_msg = "Error parsing satellite name";
    Stopif(satellite == SATFIRE_SATELLITE_NONE, goto ERR_RETURN, "Error parsing satellite name");
    clist->satellite = satellite;

    // Get the sector name
    enum Sector sector = satfire_sector_string_contains_sector(fname);
    err_msg = "Error parsing sector name";
    Stopif(sector == SATFIRE_SECTOR_NONE, goto ERR_RETURN, "Error parsing sector name");
    clist->sector = sector;

    // Get the start and end times
    clist->start = parse_time_string(cluster_find_start_time(fname));
    clist->end = parse_time_string(cluster_find_end_time(fname));

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

    clist->clusters = clusters;

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

    clist->error = true;
    clist->err_msg = err_msg;
    return clist;
}

unsigned int
cluster_list_length(struct ClusterList *list)
{
    assert(list);
    return list->clusters->len;
}

double
cluster_list_total_power(struct ClusterList *list)
{
    assert(list);

    double sum = 0.0;
    for (unsigned int i = 0; i < list->clusters->len; i++) {
        sum += g_array_index(list->clusters, struct Cluster *, i)->power;
    }

    return sum;
}
