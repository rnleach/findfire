#pragma once

#include <stdbool.h>
#include <time.h>

#include <glib.h>

#include "firepoint.h"
#include "geo.h"
#include "satellite.h"

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

/** Add a FirePoint to this Cluster. */
void cluster_add_fire_point(struct Cluster *cluster, struct FirePoint *fire_point);

/** Create a deep copy of a Cluster. */
struct Cluster *cluster_copy(struct Cluster const *cluster);

/** Get the total power of all pixels in the Cluster. */
double cluster_total_power(struct Cluster const *cluster);

/** Get the max scan angle of any pixel in this cluster. */
double cluster_max_scan_angle(struct Cluster const *cluster);

/** Get the number of SatPixels in a Cluster. */
unsigned int cluster_pixel_count(struct Cluster const *cluster);

/** Get access to the pixels in the cluster. */
const struct PixelList *cluster_pixels(struct Cluster const *cluster);

/** Get the centroid of a cluster. */
struct Coord cluster_centroid(struct Cluster const *cluster);

/** Compare Cluster objects for sorting in descending order of power. */
int cluster_descending_power_compare(const void *ap, const void *bp);

/*-------------------------------------------------------------------------------------------------
                                               ClusterList
-------------------------------------------------------------------------------------------------*/
/**
 * \brief Keep a cluster list with metadata about the file it was derived from.
 */
struct ClusterList;

/**
 * \brief Analyze a file and return a ClusterList.
 *
 * The metadata is gleaned from the file name, so this program relies on the current naming
 * conventions of the NOAA big data program.
 *
 *  \param full_path is the path to the file to analyze.
 */
struct ClusterList *cluster_list_from_file(char const *full_path);

/**
 * \brief Clean up a ClusterList object.
 *
 * After this function, the value pointed to by \a list will be set to \c 0 or \c NULL.
 */
void cluster_list_destroy(struct ClusterList **list);

/** \brief Get the satellite sector.  */
enum Sector cluster_list_sector(struct ClusterList *list);

/** \brief Get the name of the satellite. */
enum Satellite cluster_list_satellite(struct ClusterList *list);

/** Get the start time of the scan. */
time_t cluster_list_scan_start(struct ClusterList *list);

/** Get the end time of the scan. */
time_t cluster_list_scan_end(struct ClusterList *list);

/** Error status from creating the ClusterList.
 *
 * This will always be false unless there was an error creating the ClusterList. In that case the
 * cluster_list_clusters() function will return \c 0 or \c NULL and the cluster_list_error_msg()
 * function will return a message as to the source of the error.
 */
bool cluster_list_error(struct ClusterList *list);

/** The error message associated with the ClusterList.
 *
 * This is a static string determined at compile time and should not be freed.
 */
const char *cluster_list_error_msg(struct ClusterList *list);

/** Get the Clusters.
 *
 * The \c GArray holds pointers to the Cluster objects.
 */
GArray *cluster_list_clusters(struct ClusterList *list);

/** \brief Filter the ClusterList to only include fires with their centroid in the BoundingBox.
 *
 * \returns NULL on error or a reference to the same \a list that was passed in.
 */
struct ClusterList *cluster_list_filter(struct ClusterList *list, struct BoundingBox box);

/**
 * \brief Parse the file name and find the scan start time.
 */
char const *cluster_find_start_time(char const *fname);

/**
 * \brief Parse the file name and find the scan end time.
 */
char const *cluster_find_end_time(char const *fname);

/**
 * \brief Get the number of items in the ClusterList.
 */
unsigned int cluster_list_length(struct ClusterList *list);

/**
 * \brief Get the total fire power of all the clusters in this list.
 */
double cluster_list_total_power(struct ClusterList *list);
