#pragma once
/**
 * \file cluster.h
 * \brief Types and functions for working with clusters. 
 *
 * A cluster describes the aggregate properties of a connected group (or cluster) of FirePoint
 * objects.
 */

/**
 * \brief The aggregate properties of a connected group of FirePoint objects.
 */
struct Cluster {
    /// Average latitude of the points in the cluster.
    double lat;
    /// Average longitude of the points in the cluster.
    double lon;
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    double power;
    /// The number of points that are in this cluster.
    int count;
};

/** Compare \a Cluster objects for sorting in descending order of power. */
int cluster_desc_cmp(const void *ap, const void *bp);
