#pragma once

#include <stdbool.h>
#include <time.h>

#include <glib.h>

#include "geo.h"

/**
 * \file cluster.h
 * \brief Types and functions for working with clusters.
 *
 * A cluster describes the aggregate properties of a connected group (or cluster) of FirePoint
 * objects.
 */

/*-------------------------------------------------------------------------------------------------
                                                 Cluster
-------------------------------------------------------------------------------------------------*/
/**
 * \brief The aggregate properties of a connected group of FirePoint objects.
 */
struct Cluster; 

/** Create a new Cluster. */
struct Cluster *cluster_new(void);

/** Cleanup a Cluster. */
void cluster_destroy(struct Cluster **cluster);

/** Create a deep copy of a Cluster. */
struct Cluster *cluster_copy(struct Cluster *cluster);

/** Get the total power of all pixels in the Cluster. */
double cluster_total_power(struct Cluster *cluster);

/** Get a representative radius of the Cluster. */
double cluster_radius(struct Cluster *cluster);

/** Get the number of SatPixels in a Cluster. */
int cluster_pixel_count(struct Cluster *cluster);

/** Get the centroid of a cluster. */
struct Coord cluster_centroid(struct Cluster *cluster);

/** Compare Cluster objects for sorting in descending order of power. */
int cluster_descending_power_compare(const void *ap, const void *bp);

/*-------------------------------------------------------------------------------------------------
                                               ClusterList
-------------------------------------------------------------------------------------------------*/
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
char const *cluster_find_start_time(char const *fname);

/**
 * \brief Get the number of items in the ClusterList.
 */
unsigned int cluster_list_length(struct ClusterList tgt[static 1]);

/**
 * \brief Get the total fire power of all the clusters in this list.
 */
double cluster_list_total_power(struct ClusterList tgt[static 1]);
