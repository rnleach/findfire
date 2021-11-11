#pragma once
/**
 * \file firepoint.h
 * \brief All the data related to a point with fire detected.
 *
 * A FirePoint is a structure that holds all data associated with a pixel in the satellite imagery
 * that corresponds to a fire detection.
 */
#include "geo.h"

/**
 * \brief Represents all the data associated with a single pixel in which the satellite has detected
 * a fire.
 */
struct FirePoint {
    /// The x-coordinate (column number, often indexed as 'i') in the grid.
    int x;
    /// The y-coordinate (row number, often indexed as 'j') in the grid.
    int y;
    /// The polygon describing the scanned area.
    struct SatPixel pixel;
};
