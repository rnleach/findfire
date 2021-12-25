#pragma once
/** \file sf_private.h
 *
 * Internal API only, not for external use.
 */
#include "satfire.h"

#include <glib.h>

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

/** \brief Projection information required to convert from row/column number to scan angles and
 * lat-lon.
 */
struct CoordTransform {
    double xscale;  /**< Scale factor for the column for converting indexes to scan angle coords. */
    double xoffset; /**< Offset for the column for converting indexes to scan angle coords.*/
    double yscale;  /**< Scale factor for the row for converting indexes to scan angle coords.*/
    double yoffset; /**< Offset for the  row for converting indexes to scan angle coords.*/
    double req;     /**< Radius of the Earth at the equator in meters. */
    double rpol;    /**< Radius of the Earth at the poles in meters. */
    double H;       /**< Height of the satellite above the equator in meters. */
    double lon0;    /**< Longitude of the nadir point in degrees. */
};

/**
 * \brief Handle to a GDAL dataset for the Fire Detection Characteristics and some metadata.
 */
struct SatFireImage {
    /// Image width in pixels
    size_t xlen;
    /// Image height in pixels
    size_t ylen;
    /// All the information needed for transforming from row and column numbers to coordinates.
    struct CoordTransform trans;
    /// In memory buffer if this is from a zip file
    unsigned char *memory;
    /// Size of the buffer;
    size_t memory_size;
    /// Handle to the NetCDF file
    int nc_file_id;
    /// Orignial file name the dataset was loaded from.
    char fname[512];
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

/** Steal the pixels from this row.
 *
 * This is very unsafe because it leaves the struct in an invalid state with a \c NULL pointer
 * in the row->pixels. Use carefully and finalize/destroy the object soon after calling this
 * function.
 *
 * An alternative would be to call satfire_pixel_list_copy() and get a deep copy of the
 * \ref SFPixelList. But in some cases that would be a wasteful copy if we are done using the
 * \ref SFClusterRow now.
 */
struct SFPixelList *satfire_cluster_db_satfire_cluster_row_steal_pixels(struct SFClusterRow *row);
