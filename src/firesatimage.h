#pragma once

#include <stdbool.h>

#include "glib.h"
#include "gdal.h"

/**
 * \brief Handle to a GDAL dataset for the Fire Detection Characteristics and some metadata.
 */
struct FireSatImage {
    /// Orignial file name the dataset was loaded from.
    char fname[1024];
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
bool fire_sat_image_open(char const *fname, struct FireSatImage *tgt);

/** 
 * \brief Close the file and clear the pointers associated with  with this dataset.
 *
 * \param dataset is the structure to close/finalize.
 */
void fire_sat_image_close(struct FireSatImage *dataset);

/**
 * \brief Extract pixels/points from the image that have non-zero fire power.
 *
 * \returns GArray * of struct FirePoint objects.
 */
GArray *fire_sat_image_extract_fire_points(struct FireSatImage const *fdata);
