#include "satfire.h"

#include <assert.h>
#include <stdbool.h>
#include <string.h>

#include "sf_private.h"
#include "sf_util.h"

char const *out_of_memory = "memory allocation error";

/*-------------------------------------------------------------------------------------------------
                                                 Cluster
-------------------------------------------------------------------------------------------------*/
struct SFCluster {
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    double power;
    /// Total (sum) of the fire area of the points in the cluster with area in square meters.
    double area;
    /// Maximum temperature of all the pixels in the cluster in Kelvin.
    double max_temp;
    /// The maximum scan angle of any point in this cluster
    double max_scan_angle;
    /// Pixels making up the cluster.
    struct SFPixelList *pixels;
};

struct SFCluster *
satfire_cluster_new(void)
{
    struct SFCluster *new = malloc(sizeof(struct SFCluster));
    Stopif(!new, exit(EXIT_FAILURE), "%s", out_of_memory);

    *new =
        (struct SFCluster){.power = 0.0, .pixels = satfire_pixel_list_new(), .max_scan_angle = 0.0};

    return new;
}

void
satfire_cluster_destroy(struct SFCluster **cluster)
{
    assert(cluster);
    assert(*cluster);
    (*cluster)->pixels = satfire_pixel_list_destroy((*cluster)->pixels);
    free(*cluster);
    *cluster = 0;
}

void
satfire_cluster_add_fire_point(struct SFCluster *cluster, struct FirePoint *fire_point)
{
    assert(cluster);
    assert(fire_point);

    cluster->pixels = satfire_pixel_list_append(cluster->pixels, &fire_point->pixel);
    if (!isinf(fire_point->pixel.power)) {
        cluster->power += fire_point->pixel.power;
    }

    if (!isinf(fire_point->pixel.temperature)) {
        cluster->max_temp = fmax(cluster->max_temp, fire_point->pixel.temperature);
    }

    if (!isinf(fire_point->pixel.area)) {
        cluster->area += fire_point->pixel.area;
    }

    cluster->max_scan_angle = fmax(cluster->max_scan_angle, fire_point->pixel.scan_angle);
}

struct SFCluster *
satfire_cluster_copy(struct SFCluster const *cluster)
{
    assert(cluster);

    struct SFCluster *copy = malloc(sizeof(struct SFCluster));
    Stopif(!copy, exit(EXIT_FAILURE), "%s", out_of_memory);

    *copy = (struct SFCluster){.power = cluster->power,
                               .area = cluster->area,
                               .max_temp = cluster->max_temp,
                               .pixels = satfire_pixel_list_copy(cluster->pixels),
                               .max_scan_angle = cluster->max_scan_angle};

    return copy;
}

double
satfire_cluster_total_power(struct SFCluster const *cluster)
{
    assert(cluster);
    return cluster->power;
}

double
satfire_cluster_total_area(struct SFCluster const *cluster)
{
    assert(cluster);
    return cluster->area;
}

double
satfire_cluster_max_temperature(struct SFCluster const *cluster)
{
    assert(cluster);
    return cluster->max_temp;
}

double
satfire_cluster_max_scan_angle(struct SFCluster const *cluster)
{
    assert(cluster);
    return cluster->max_scan_angle;
}

unsigned int
satfire_cluster_pixel_count(struct SFCluster const *cluster)
{
    assert(cluster);

    return cluster->pixels->len;
}

const struct SFPixelList *
satfire_cluster_pixels(struct SFCluster const *cluster)
{
    assert(cluster);
    return cluster->pixels;
}

struct SFCoord
satfire_cluster_centroid(struct SFCluster const *cluster)
{
    assert(cluster);
    return satfire_pixel_list_centroid(cluster->pixels);
}

int
satfire_cluster_descending_power_compare(const void *ap, const void *bp)
{
    struct SFCluster const *a = ap;
    struct SFCluster const *b = bp;

    if (a->power > b->power)
        return -1;
    if (a->power < b->power)
        return 1;
    return 0;
}
/*-------------------------------------------------------------------------------------------------
                                               ClusterList
-------------------------------------------------------------------------------------------------*/
struct SFClusterList {
    enum SFSector sector;
    enum SFSatellite satellite;
    time_t start;     /**< Start time of the scan. */
    time_t end;       /**< End time of the scan*/
    GArray *clusters; /**< List of struct SFCluster objects associated with the above metadata. */
    char *err_msg;    /**< Error message. */
    bool error;       /**< Error flag. False indicates no error. */
};

void
satfire_cluster_list_destroy(struct SFClusterList **list)
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

enum SFSector
satfire_cluster_list_sector(struct SFClusterList *list)
{
    assert(list);
    return list->sector;
}

enum SFSatellite
satfire_cluster_list_satellite(struct SFClusterList *list)
{
    assert(list);
    return list->satellite;
}

time_t
satfire_cluster_list_scan_start(struct SFClusterList *list)
{
    assert(list);
    return list->start;
}

time_t
satfire_cluster_list_scan_end(struct SFClusterList *list)
{
    assert(list);
    return list->end;
}

bool
satfire_cluster_list_error(struct SFClusterList *list)
{
    assert(list);
    return list->error;
}

const char *
satfire_cluster_list_error_msg(struct SFClusterList *list)
{
    assert(list);
    return list->err_msg;
}

GArray *
satfire_cluster_list_clusters(struct SFClusterList *list)
{
    assert(list);

    if (list->error) {
        assert(!list->clusters); // Force to be NULL
    }

    return list->clusters;
}

struct SFClusterList *
satfire_cluster_list_filter_box(struct SFClusterList *list, struct SFBoundingBox box)
{
    assert(list);

    GArray *clusters = list->clusters;

    for (unsigned int i = 0; i < clusters->len; ++i) {
        struct SFCluster *clust = g_array_index(clusters, struct SFCluster *, i);

        struct SFCoord centroid = satfire_cluster_centroid(clust);

        if (!satfire_bounding_box_contains_coord(box, centroid, 0.0)) {
            clusters = g_array_remove_index_fast(clusters, i);
            --i; // Decrement this so we inspect this index again since a new value is there.
        }
    }

    list->clusters = clusters; // In case g_array_remove_index_fast() moved the array.

    return list;
}

struct SFClusterList *
satfire_cluster_list_filter_scan_angle(struct SFClusterList *list, double max_scan_angle)
{
    assert(list);

    GArray *clusters = list->clusters;

    for (unsigned int i = 0; i < clusters->len; ++i) {
        struct SFCluster *clust = g_array_index(clusters, struct SFCluster *, i);

        double clust_max_scan_angle = satfire_cluster_max_scan_angle(clust);

        if (clust_max_scan_angle >= max_scan_angle) {
            clusters = g_array_remove_index_fast(clusters, i);
            --i; // Decrement this so we inspect this index again since a new value is there.
        }
    }

    list->clusters = clusters; // In case g_array_remove_index_fast() moved the array.

    return list;
}

char const *
satfire_cluster_find_start_time(char const *fname)
{
    char const *start = strstr(fname, "_s");
    if (start)
        return start + 2;
    return start;
}

char const *
satfire_cluster_find_end_time(char const *fname)
{
    char const *end = strstr(fname, "_e");
    if (end)
        return end + 2;
    return end;
}

static void
local_satfire_cluster_destroy(void *cluster)
{
    struct SFCluster **clst = cluster;
    satfire_cluster_destroy(clst);
}

static GArray *
clusters_from_fire_points(GArray const *points)
{
    GArray *clusters = g_array_sized_new(false, true, sizeof(struct SFCluster *), 100);
    g_array_set_clear_func(clusters, local_satfire_cluster_destroy);

    GArray *satfire_cluster_points = g_array_sized_new(false, true, sizeof(struct FirePoint), 20);

    for (unsigned int i = 0; i < points->len; i++) {

        struct FirePoint *fp = &g_array_index(points, struct FirePoint, i);

        if (fp->x == 0 && fp->y == 0)
            continue;

        satfire_cluster_points = g_array_append_val(satfire_cluster_points, *fp);
        fp->x = 0;
        fp->y = 0;

        for (unsigned int j = i + 1; j < points->len; j++) {
            struct FirePoint *candidate = &g_array_index(points, struct FirePoint, j);

            if (candidate->x == 0 && candidate->y == 0)
                continue;
            for (unsigned int k = 0; k < satfire_cluster_points->len; ++k) {
                struct FirePoint *a_point_in_cluster =
                    &g_array_index(satfire_cluster_points, struct FirePoint, k);

                int dx = abs(a_point_in_cluster->x - candidate->x);
                int dy = abs(a_point_in_cluster->y - candidate->y);

                if (dx <= 1 && dy <= 1) {
                    satfire_cluster_points = g_array_append_val(satfire_cluster_points, *candidate);
                    candidate->x = 0;
                    candidate->y = 0;
                }
            }
        }

        struct SFCluster *curr_clust = satfire_cluster_new();
        struct FirePoint *curr_fire_point =
            &g_array_index(satfire_cluster_points, struct FirePoint, 0);
        satfire_cluster_add_fire_point(curr_clust, curr_fire_point);

        for (unsigned int j = 1; j < satfire_cluster_points->len; ++j) {

            curr_fire_point = &g_array_index(satfire_cluster_points, struct FirePoint, j);
            satfire_cluster_add_fire_point(curr_clust, curr_fire_point);
        }

        clusters = g_array_append_val(clusters, curr_clust);
        satfire_cluster_points = g_array_set_size(satfire_cluster_points, 0);
    }

    g_array_unref(satfire_cluster_points);

    return clusters;
}

struct SFClusterList *
satfire_cluster_list_from_file(char const *full_path)
{
    struct SFClusterList *clist = calloc(1, sizeof(struct SFClusterList));
    char *err_msg = 0;
    GArray *points = 0;
    GArray *clusters = 0;

    char const *fname = get_file_name(full_path);

    // Get the satellite
    enum SFSatellite satellite = satfire_satellite_string_contains_satellite(fname);
    err_msg = "Error parsing satellite name";
    Stopif(satellite == SATFIRE_SATELLITE_NONE, goto ERR_RETURN, "Error parsing satellite name");
    clist->satellite = satellite;

    // Get the sector name
    enum SFSector sector = satfire_sector_string_contains_sector(fname);
    err_msg = "Error parsing sector name";
    Stopif(sector == SATFIRE_SECTOR_NONE, goto ERR_RETURN, "Error parsing sector name");
    clist->sector = sector;

    // Get the start and end times
    clist->start = parse_time_string(satfire_cluster_find_start_time(fname));
    clist->end = parse_time_string(satfire_cluster_find_end_time(fname));

    // Get the clusters member.
    struct SatFireImage fdata = {0};
    bool ok = fire_sat_image_open(full_path, &fdata);
    Stopif(!ok, err_msg = "Error opening NetCDF file";
           goto ERR_RETURN, "Error opening NetCDF file %s", full_path);

    points = fire_sat_image_extract_fire_points(&fdata);
    fire_sat_image_close(&fdata);
    Stopif(!points, goto ERR_RETURN, "Error extracting fire points.");

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
satfire_cluster_list_length(struct SFClusterList *list)
{
    assert(list);
    return list->clusters->len;
}

double
satfire_cluster_list_total_power(struct SFClusterList *list)
{
    assert(list);

    double sum = 0.0;
    for (unsigned int i = 0; i < list->clusters->len; i++) {
        sum += g_array_index(list->clusters, struct SFCluster *, i)->power;
    }

    return sum;
}
