#pragma once
/**
 * \file geo.h
 *
 *  \brief Geographic types and calculations.
 *
 * For the purpose of working with GOES-R/S satellite imagery working with quadrilaterals
 * representing the area of a scan pixel on earth as viewed from the satellite is all that is
 * necessary. A general purpose GIS library, such as GDAL, is not necessary. During prototyping with
 * a general purpose GIS library, it actually proved to be very problematic. The nature of floating
 * point numbers combined with the nature of working with so many adjacent pixels caused more
 * problems than a general GIS library could handle. This type of dataset is rife with edge cases
 * that were difficult to handle.
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

/** \brief Determine if these coordinates are close to each other.
 *
 * The \a eps parameter is the maximum distance between points in the same units as the
 * coordinates that two points can have and still be considered close.
 */
bool coord_are_close(struct Coord left, struct Coord right, double eps);

/*-------------------------------------------------------------------------------------------------
 *                                         SatPixels
 *-----------------------------------------------------------------------------------------------*/
/** The coordinates describing the area of a pixel viewed from a GOES-R/S satellite. */
struct SatPixel {
    union {
        struct {
            struct Coord ul;
            struct Coord ll;
            struct Coord lr;
            struct Coord ur;
        };
        struct Coord coords[4];
    };
};

/** Calculate the centroid of a SatPixel.
 *
 * This function uses an algorithm that assumes the pixel is a quadrilateral, which is enforced
 * by the definition of the SatPixel type.
 */
struct Coord sat_pixel_centroid(struct SatPixel pxl[static 1]);

/** Tests if these pixels are basically the same pixel.
 *
 * This compares the four corners of the pixel using the coord_are_close() function.
 */
bool sat_pixels_approx_equal(struct SatPixel left[static 1], struct SatPixel right[static 1],
                             double eps);

/** Determine if a coordinate is interior to a pixel.
 *
 * Interior means that it is NOT on the boundary. The eps parameter is used by an interanl line
 * intersection function to detect if the intersection point is very close to an end point.
 */
bool sat_pixel_contains_coord(struct SatPixel const pxl[static 1], struct Coord coord, double eps);

/** Determine if satellite pixels overlap.
 *
 * Overlapping is defined as one pixel having a vertex / corner that is interior to the other one
 * or as pixels having edges that intersect.
 *
 * The eps parameter is used as a parameter for any cases where floating point values need to be
 * compared. There are a few places in the algorithm where that happens, and if they are within
 * eps units of each other, they are considered equal.
 */
bool sat_pixels_overlap(struct SatPixel left[static 1], struct SatPixel right[static 1],
                        double eps);

/** Determine if satellite pixels are adjacent.
 *
 * Adjacent is defined as having at least one corner that is 'eps' close to a coordinate in the
 * other. Adjacent pixels may overlap also because sat_pixels_overlap() uses the eps variable in
 * determining overlap. However, if there is a large overlap, the pixels aren't adjacent.
 *
 * \param left a satellite pixel to check.
 * \param right the pixel to check against.
 * \param eps The scale to use for comparison in the same units as the lat and lon.
 **/
bool sat_pixels_are_adjacent(struct SatPixel left[static 1], struct SatPixel right[static 1],
                             double eps);

/*-------------------------------------------------------------------------------------------------
 *                                         PixelList
 *-----------------------------------------------------------------------------------------------*/
/** A pixel list stores a list of SatPixel objects. */
struct PixelList {
    size_t len;
    size_t capacity;
    struct SatPixel pixels[];
};

/** Create a new PixelList. */
struct PixelList *pixel_list_new();

/** Create a new PixelList with a given capacity. */
struct PixelList *pixel_list_new_with_capacity(size_t capacity);

/** Destroy a PixelList.  */
struct PixelList *pixel_list_destroy(struct PixelList plist[static 1]);

/** Create a deep copy of the PixelList. */
struct PixelList *pixel_list_copy(struct PixelList plist[static 1]);

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
 * After this call the list is basically in the same state as after calling pixel_list_new().
 */
struct PixelList *pixel_list_clear(struct PixelList list[static 1]);

/** Calculate the centroid of a PixelList. */
struct Coord pixel_list_centroid(struct PixelList list[static 1]);

/*-------------------------------------------------------------------------------------------------
 *                                         Binary Format
 *-----------------------------------------------------------------------------------------------*/
// The binary formatting is for storing or retrieving from a database.

/** Calculate the amount of space needed in a buffer to encode a PixelList into binary. */
size_t pixel_list_binary_serialize_buffer_size(struct PixelList const plist[static 1]);

/** Encode the PixelList into a binary format suitable for storing in a database.
 *
 * At this time it doesn't support a portable format, meaning no corrections are made for
 * endianness or any padding in the array.
 *
 * \return The number of bytes written.
 */
size_t pixel_list_binary_serialize(struct PixelList const plist[static 1], size_t buf_size,
                                   unsigned char buffer[buf_size]);

/** Deserialize an array of bytes into a PixelList.
 *
 * \return an allocated PixelList that should be cleaned up with pixel_list_destroy(). In the
 * event of an error, it returns NULL.
 */
struct PixelList *pixel_list_binary_deserialize(unsigned char buffer[static sizeof(size_t)]);

/*-------------------------------------------------------------------------------------------------
 *                                         KML Export
 *-----------------------------------------------------------------------------------------------*/
/** Write out a pixel list in KML format.
 *
 * This will print out a multigeometry KML element. It should be composed as part of a function
 * that outputs a KML file where that higher function adds style information and the rest of the
 * document.
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
