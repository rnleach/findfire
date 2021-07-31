#include "cluster.h"

#include <stdbool.h>

#include "firepoint.h"

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
