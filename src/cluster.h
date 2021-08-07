#pragma once

#include <stdbool.h>
#include <time.h>

#include <glib.h>

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
    /// The distance from the cluster center to the farthest point in the cluster.
    double radius;
    /// The number of points that are in this cluster.
    int count;
};

/**
 * \brief Keep a cluster list with metadata about the file it was derived from.
 *
 * If there is an error, the error member will be true, there will be an error message, and the
 * clusters pointer will be set to null.
 */
struct ClusterList {
    /// This is the sector, "FDCC", "FDCF", or "FDCM"
    ///
    /// FDCC is the CONUS scale
    /// FDCF is the full disk scale
    /// FDCM is the mesosector scale
    char sector[5];
    /// This is the source satellite.
    ///
    /// At the time of writing it will either be "G16" or "G17"
    char satellite[4];
    /// Start time of the scan
    time_t start;
    /// End time of the scan
    time_t end;
    /// List of struct Cluster objects associated with the above metadata.
    GArray *clusters;
    /// Error message.
    char *err_msg;
    /// Error flag. False indicates no error.
    bool error;
};

/** Compare \a Cluster objects for sorting in descending order of power. */
int cluster_desc_cmp(const void *ap, const void *bp);

/**
 * \brief Group struct FirePoint objects into clusters.
 *
 * FirePoint objects that are directly adjacent to each other are grouped into clusters where
 * each point is in direct contact with at least one other point in the cluster.
 *
 * \param points is an array of struct FirePoint objects.
 *
 * \returns an array of struct Cluster objects.
 * */
GArray *clusters_from_fire_points(GArray const *points);

/**
 * \brief Analyze a file and return a ClusterList including the file metadata.
 *
 * The metadata is gleaned from the file name at this time.
 *
 *  \param full_path is the path to the file to analyze.
 */
struct ClusterList cluster_list_from_file(char const *full_path);

/**
 * \brief Clean up a struct ClusterList object.
 *
 * Use this function to clean up a ClusterList object. After it's cleaned up, it will be as if
 * memset(0, sizeof(struct ClusterList)) had been called on the struct. This is meant to be used
 * for all ClusterList objects regardless of error state.
 */
void cluster_list_clear(struct ClusterList *tgt);

/**
 * \brief Parse the file name and find the scan start time.
 */
char const* cluster_find_start_time(char const *fname);
