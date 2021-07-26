#pragma once
/**
 * \file firepoint.h
 * \brief All the data related to a point with fire detected.
 *
 * A FirePoint is a structure that holds all data associated with a pixel in the satellite imagery
 * that corresponds to a fire detection.
 */

/** 
 * \brief Represents all the data associated with a single pixel in which the satellite has detected
 * a fire.
 */
struct FirePoint {
    /// The x-coordinate (column number, often indexed as 'i') in the grid.
    int x;
    /// The y-coordinate (row number, often indexed as 'j') in the grid.
    int y;
    /// The latitude
    float lat;
    /// The longitude
    float lon;
    /// The power of the fire in that pixel in megawatts.
    float power;
};

/**
 * \brief Calculate the great circle distance between two \a FirePoint objects.
 *
 * Calculate the great circle distance between \a a and \a b.
 *
 * \return The distance between the points in meters.
 */
double firepoint_great_circle_distance(struct FirePoint const a, struct FirePoint const b);

