#pragma once
/**
 * \file geo.h
 *
 *  Geographic types and calculations.
 *
 * For the purpose of working with GOES-R/S satellite imagery working with quadrilaterals
 * representing the area of a scan pixel on earth as viewed from the satellite is all that is
 * necessary. A general purpose GIS library, such as GDAL, is not necessary. During prototyping,
 * using a general purpose GIS library actually proved to be very problematic. The nature of
 * floating point numbers combined with the nature of working with so many adjacent pixels caused
 * more problems than a general GIS library could handle. The particular dataset we are working with
 * is rife with edge cases that were difficult to handle.
 *
 * To deal with those edge cases, this module deals directly with the approximate equality of
 * floating point values.
 */

#include <math.h>
#include <stdbool.h>
#include <stdio.h>

/*-------------------------------------------------------------------------------------------------
 *                                         Coordinates
 *-----------------------------------------------------------------------------------------------*/
/** A coordinate consisting of a latitude and a longitude. */
struct Coord {
    double lat;
    double lon;
};

/** Determine if these coordinates are close to each other.
 *
 * The \param eps parameter is the maximum distance between points in the same units as the
 * coordinates that two points can have and still be considered close.
 */
bool coord_are_close(struct Coord left, struct Coord right, double eps);

/*-------------------------------------------------------------------------------------------------
 *                                         SatPixels
 *-----------------------------------------------------------------------------------------------*/
/** The coordinates describing the area of a pixel viewed from a GOES-R/S satellite. */
struct SatPixel {
    struct Coord ul;
    struct Coord ll;
    struct Coord lr;
    struct Coord ur;
};

/** Calculate the centroid of a SatPixel.
 *
 * This function uses an algorithm that assumes the pixel is a quadrilateral, which is enforced
 * by the definition of the \a SatPixel type.
 */
struct Coord sat_pixel_centroid(struct SatPixel pxl[static 1]);

/** Are these pixels basically the same pixel. */
bool sat_pixels_approx_equal(struct SatPixel left[static 1], struct SatPixel right[static 1],
                             double eps);

/** Determine if satellite pixels are adjacent.
 *
 * Adjacent is defined as having at least one corner that is close to a coordinate in the other.
 *
 * \param left a satellite pixel to check.
 * \param right the pixel to check against.
 * \param eps The scale to use for comparison in the same units as the lat and lon.
 **/
bool sat_pixels_are_adjacent(struct SatPixel left[static 1], struct SatPixel right[static 1],
                             double eps);

/** Determine if a coordinate is interior to a pixel. */
bool sat_pixel_contains_coord(struct SatPixel pxl[static 1], struct Coord coord[static 1]);

/** Determine if satellite pixels overlap.
 *
 * Overlapping is defined as one pixel having a vertex / corner that is interior to the other one.
 */
bool sat_pixels_overlap(struct SatPixel left[static 1], struct SatPixel right[static 1]);

/*-------------------------------------------------------------------------------------------------
 *                                         PixelList
 *-----------------------------------------------------------------------------------------------*/
/** A pixel list stores a list of SatPixel objects. */
struct PixelList {
    size_t capacity;
    size_t length;
    struct SatPixel pixels[];
};

/** Create a new PixelList. */
struct PixelList *pixel_list_new();

/** Create a new PixelList with a given capacity. */
struct PixelList *pixel_list_new_with_capacity(size_t capacity);

/** Destroy a PixelList.
 *
 * Destroy the PixelList an nullify the pointer.
 */
void pixel_list_destroy(struct PixelList *plist[static 1]);

/** Append a SatPixel to the list.
 *
 * Reallocates the backing array if necessary and returns the new pointer, so always use the return
 * value as the new list. If the system is running out of memory and the allocation fails, it
 * aborts the program.
 *
 * \return A (potentially new) pointer to the list \param plist.
 */
struct PixelList *pixel_list_append(struct PixelList list[static 1],
                                    struct SatPixel apix[static 1]);

/** Clear the list but keep the memory in tact.
 *
 * After this call the list is basically in the same state as after calling \a pixel_list_new.
 */
void pixel_list_clear(struct PixelList list[static 1]);

/*-------------------------------------------------------------------------------------------------
 *                                         Binary Format
 *-----------------------------------------------------------------------------------------------*/
// The binary formatting is for storing or retrieving from a database.

/** Calculate the amount of space needed in a buffer to encode a PixelList into binary. */
size_t pixel_list_binary_serialize_buffer_size(struct PixelList plist[static 1]);

/** Encode the PixelList into a binary format suitable for storing in a database.
 *
 * At this time it doesn't support a portable format, meaning no corrections are made for
 * endianness or any padding in the array.
 *
 * \return The number of bytes written.
 */
size_t pixel_list_binary_serialize(struct PixelList plist[static 1], size_t buf_size,
                                   unsigned char buffer[buf_size]);

/** Deserialize an array of bytes into a PixelList.
 *
 * \return an allocated PixelList that should be cleaned up with pixel_list_destroy(). In the
 * event of an error, it returns NULL.
 */
struct PixelList *pixel_list_binary_deserialize(size_t buf_size, unsigned char buffer[buf_size]);

/*-------------------------------------------------------------------------------------------------
 *                                         KML Export
 *-----------------------------------------------------------------------------------------------*/
/** Write out a pixel list in KML format.
 *
 * This will print out a multigeometry KML element. It should be composed as part of a function
 * that outputs a KML file where that higher function add style information.
 *
 * \returns the number of bytes written to the \param strm or -1 if there was an error.
 */
int pixel_list_kml_print(FILE *strm, struct PixelList plist[static 1]);

/*-------------------------------------------------------------------------------------------------
 *                                            Misc
 *-----------------------------------------------------------------------------------------------*/
/**
 * \brief the simple great circle distance calculation.
 *
 * \param lat1 the latitude of the first point in degrees.
 * \param lon1 the longitude of the first point in degrees.
 * \param lat2 the latitude of the second point in degrees.
 * \param lon2 the longitude of the second point in degrees.
 *
 * \return the distance between the points in kilometers.
 */
double great_circle_distance(double lat1, double lon1, double lat2, double lon2);
