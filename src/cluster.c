#include "cluster.h"

int
cluster_desc_cmp(const void *ap, const void *bp)
{
    struct Cluster const *a = ap;
    struct Cluster const *b = bp;

    if (a->power > b->power) return -1;
    if (a->power < b->power) return  1;
    return 0;
}

