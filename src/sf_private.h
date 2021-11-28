#pragma once
/** \file sf_private.h
 *
 * Internal API only, not for external use.
 */
#include "satfire.h"

#include "gdal.h"
#include "glib.h"

/**
 * \brief Represents all the data associated with a single pixel in which the satellite has detected
 * a fire.
 */
struct FirePoint {
    /// The polygon describing the scanned area.
    struct SFPixel pixel;
    /// The x-coordinate (column number, often indexed as 'i') in the grid.
    int x;
    /// The y-coordinate (row number, often indexed as 'j') in the grid.
    int y;
};

/**
 * \brief Handle to a GDAL dataset for the Fire Detection Characteristics and some metadata.
 */
struct SatFireImage {
    /// Orignial file name the dataset was loaded from.
    char fname[512];
    /// Handle to the dataset.
    GDALDatasetH dataset;
    /// Transform from array indexes to spatial coordinates.
    double geo_transform[6];
    /// Handle to the desired band from the dataset
    GDALRasterBandH band;
    /// The size of a row (x-dimension) in the band.
    int x_size;
    /// The number or rows (y-dimension) in the band.
    int y_size;
};

/**
 * \brief Open a file containing GOES-R/S Fire Detection Characteristics.
 *
 * \param fname the path to the file name to open.
 * \param tgt the [FireDataSet] structure to initialize.
 *
 * \returns false if there is an error opening the data.
 */
bool fire_sat_image_open(char const *fname, struct SatFireImage *tgt);

/**
 * \brief Close the file and clear the pointers associated with  with this dataset.
 *
 * \param dataset is the structure to close/finalize.
 */
void fire_sat_image_close(struct SatFireImage *dataset);

/**
 * \brief Extract pixels/points from the image that have non-zero fire power.
 *
 * \returns GArray * of struct FirePoint objects.
 */
GArray *fire_sat_image_extract_fire_points(struct SatFireImage const *fdata);

/** Add a FirePoint to this Cluster. */
void satfire_cluster_add_fire_point(struct SFCluster *cluster, struct FirePoint *fire_point);
