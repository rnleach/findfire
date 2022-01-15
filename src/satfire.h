#pragma once
/** \file satfire.h
 *
 * \brief Library for working with Fire Data Characterization datasets from GOES-16/17 and the NOAA
 * Big Data project.
 *
 * <h3>Metadata about the satellite platforms.</h3>
 * \ref SFSatellite<br/>
 * \ref SFSector<br/>
 *
 * These enumerations and associated functions are for working with the current field of operating
 * GOES satellites.
 *
 * <h3>Geographic types and calculations.</h3>
 * \ref SFCoord<br/>
 * \ref SFBoundingBox<br/>
 * \ref SFPixel<br/>
 * \ref SFPixelList<br/>
 *
 * For the purpose of working with GOES-R/S satellite imagery working with quadrilaterals
 * representing the area of a scan pixel on earth as viewed from the satellite is all that is
 * necessary. A general purpose GIS library, such as GDAL, is not necessary. During prototyping with
 * a general purpose GIS library, it actually proved to be very problematic. The nature of floating
 * point numbers combined with the nature of working with so many adjacent pixels caused more
 * problems than a general GIS library could handle. This type of dataset is rife with edge cases
 * that were difficult to handle. To deal with those edge cases, this library deals directly with
 * the approximate equality of floating point values.
 *
 * The \ref SFPixel type includes quite a bit of data about a satellite pixel. One of the values
 * is the scan angle, which is the angle between the pixel centroid and the satellites nadir point
 * relative to the center of the Earth. This may seem like a rather random metric, but it's a good
 * proxy for how close the pixel is to the limb of the Earth as viewed by the satellite, and it's
 * easy to calculate. Early investigations suggest that this is likely important data for quality
 * control and assessing the validity of a detection.
 *
 * There are also functions for serializing and deserializing a SFPixelList in a <b>non-portable
 * binary</b> format as well as a function to serialize it as a KML Placemark using the kamel
 * library.
 *
 * IMPORTANT: The types and functions in this library do not handle geographic features that
 * straddle the international date line correctly. But this doesn't really come up in our use case.
 *
 * <h3>Clusters.</h3>
 * \ref SFCluster<br/>
 * \ref SFClusterList<br/>
 *
 * This group of types and their associated functions are for loading data from files and detecting
 * spatially connected clusters of satellite pixels where fire power was detected / analyzed. These
 * functions depend on the file naming convention from the NOAA Big Data project to detect certain
 * metadata such as the satellite, scan sector, and start and end times of the scans. There is also
 * some functions that can be used to filter lists based on cluster properties.
 *
 * <h3>The Cluster Database</h3>
 * \ref SFDatabaseH<br/>
 * \ref SFClusterDatabaseAddH<br/>
 * \ref SFClusterDatabaseQueryPresentH<br/>
 * \ref SFClusterDatabaseQueryRowsH<br/>
 * \ref SFClusterRow<br/>
 *
 * This group of types and their associated functions are for storing and querying clusters from a
 * database.
 *
 */
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <time.h>

#include <glib.h>

/** \brief Initialize the library.
 *
 * This should be run once in the main thread before calling any other satfire functions and before
 * any other threads are spawned.
 */
void satfire_initialize(void);

/** \brief Clean up the resources associated with the library.
 *
 * This should be run once in the main thread at the end of the program once all other threads have
 * been shutdown.
 */
void satfire_finalize(void);

/** \brief The GOES satellites this library works with. */
enum SFSatellite {
    SATFIRE_SATELLITE_G16, /**< GOES-16, formerly GOES-R. */
    SATFIRE_SATELLITE_G17, /**< GOES-17, formerly GOES-S. */
    SATFIRE_SATELLITE_NUM, /**< The number of satellites in the enumeration. */
    SATFIRE_SATELLITE_NONE = SATFIRE_SATELLITE_NUM /**< Used as an error code. */
};

/** \brief The satellite scan sectors this library recognizes. */
enum SFSector {
    SATFIRE_SECTOR_FULL,  /**< The full disk scan sector. */
    SATFIRE_SECTOR_CONUS, /**< The CONUS, Continental U.S. scan sector. */
    SATFIRE_SECTOR_MESO1, /**< There are two floating meso sectors. */
    SATFIRE_SECTOR_MESO2, /**< There are two floating meso sectors. */
    SATFIRE_SECTOR_NUM,   /**< The number of valid members in this enumeration. */
    SATFIRE_SECTOR_NONE = SATFIRE_SECTOR_NUM /**< Used as an error code. */
};

/** \brief Get a string representing the name of the satellite. */
char const *satfire_satellite_name(enum SFSatellite const sat);

/** \brief Scan the string for the occurrence of a satellite name and return the first one found.
 *
 * \returns SFSatellite that corresponds to the first satellite name found, or
 * SATFIRE_SATELLITE_NONE if none was found.
 */
enum SFSatellite satfire_satellite_string_contains_satellite(char const *str);

/** \brief Get the earliest operational date for the satellite. */
time_t satfire_satellite_operational(enum SFSatellite const sat);

/** \brief Get a string representing the sector. */
char const *satfire_sector_name(enum SFSector const sector);

/** \brief Scan the string for the occurrence of a sector name and return the first one found.
 *
 * \returnsSFSector that corresponds to the first sector name found, or SATFIRE_SECTOR_NONE
 * if none was found.
 */
enum SFSector satfire_sector_string_contains_sector(char const *str);

/** \brief Translate a mask code to a string.
 *
 * Mask codes are a form of metadata that describe each pixel's quality control characteristics.
 * These codes were taken from table 5.19.6.1-1 of the GOES-R SERIES PRODUCT DEFINITION AND USERS’
 * GUIDE retrieved December 10th, 2021 from https://www.goes-r.gov/products/docs/PUG-L2+-vol5.pdf
 */
char const *satfire_satellite_mask_code_to_string(short code);

/** \brief Translate a data quality flag (DQF) code to a string.
 *
 * DQF codes are a simplified version of the mask codes described above that only tell the result
 * of the quality control analysis.  These codes were taken from table 5.19.6.1-2 of the GOES-R
 * SERIES PRODUCT DEFINITION AND USERS’ GUIDE retrieved December 10th, 2021 from
 * https://www.goes-r.gov/products/docs/PUG-L2+-vol5.pdf
 */
char const *satfire_satellite_dqf_code_to_string(signed short code);

/*-------------------------------------------------------------------------------------------------
 *                                         Coordinates
 *-----------------------------------------------------------------------------------------------*/
/** A coordinate consisting of a latitude and a longitude. */
struct SFCoord {
    double lat; /**< Latitude. Should be -90 to 90, but that's not checked or enforced.    */
    double lon; /**< Longitude. Should be -180 to 180, but that's not checked or enforced. */
};

/** \brief Determine if these coordinates are close to each other.
 *
 * The \a eps parameter is the maximum distance between points in the same units as the
 * coordinates that two points can have and still be considered close.
 */
bool satfire_coord_are_close(struct SFCoord left, struct SFCoord right, double eps);

/*-------------------------------------------------------------------------------------------------
 *                                       BoundingBox
 *-----------------------------------------------------------------------------------------------*/
/** \brief Represents a "square" area in latitude-longitude coordinates. */
struct SFBoundingBox {
    struct SFCoord ll; /**< The lower left corner of the box.  */
    struct SFCoord ur; /**< The upper right corner of the box. */
};

/** \brief Check to see if a Coord is inside of a BoundingBox.
 *
 * \param box is the bounding box in question.
 * \param coord is the coordinate, or point, in question.
 * \param eps is a fuzzy factor. Any point 'eps' close to the box will be considered internal as
 * well. If \a eps is 0.0, then the comparison is exact.
 *
 * \returns \c true if the point \a coord is interior to the box.
 */
bool satfire_bounding_box_contains_coord(struct SFBoundingBox const box, struct SFCoord const coord,
                                         double eps);

/*-------------------------------------------------------------------------------------------------
 *                                         SFPixels
 *-----------------------------------------------------------------------------------------------*/
/** The coordinates describing the area of a pixel viewed from a GOES-R/S satellite. */
struct SFPixel {
    /// The corner points of the pixel.
    union {
        struct {
            struct SFCoord ul;
            struct SFCoord ll;
            struct SFCoord lr;
            struct SFCoord ur;
        };
        struct SFCoord coords[4];
    };
    /// The radiative power in MegaWatts in this pixel.
    double power;
    /// The estimated area of the pixel covered by the fire in square meters.
    double area;
    /// The estimated temperature of the fire in K
    double temperature;
    /// This is the scan angle as measured in the coordinate system of the satellite. The satellite
    /// measures the x and y positions of a pixel on a grid by the angle each makes with the central
    /// point which looks at nadir on the Earth. There are two values, an x scan angle and a y scan
    /// angle. They are combined via the Euclidian norm sqrt(x^2 + y^2) to form the scan_angle.
    ///
    /// Constant values of the scan angle form concentric circles around the nadir point on the
    /// Earth's surface. All points along that line have a very similar (equal if the Earth was a
    /// sphere) angle between the satellites view and the local zenith. This is a good proxy for
    /// how much of an edge on vs straight down view, which can be useful for quality control.
    double scan_angle;
    /// Mask is a code that describes the outcome of the algorithms that characterize a fire point.
    ///
    /// See the satfire_satellite_mask_code_to_string() function for reference.
    short mask_flag;
    /// Data Quality Flag
    ///
    /// See the satfire_satellite_dqf_code_to_string() function for reference.
    signed short data_quality_flag;
};

/** Calculate the centroid of a SFPixel.
 *
 * This function uses an algorithm that assumes the pixel is a quadrilateral, which is enforced
 * by the definition of the SFPixel type.
 */
struct SFCoord satfire_pixel_centroid(struct SFPixel const pxl[static 1]);

/** Tests if these pixels are basically the same pixel in a geographic sense (not including power).
 *
 * This compares the four corners of the pixel using the satfire_coord_are_close() function.
 */
bool satfire_pixels_approx_equal(struct SFPixel const left[static 1],
                                 struct SFPixel const right[static 1], double eps);

/** Determine if a coordinate is interior to a pixel.
 *
 * Interior means that it is NOT on the boundary. The eps parameter is used by an interanl line
 * intersection function to detect if the intersection point is very close to an end point.
 */
bool satfire_pixel_contains_coord(struct SFPixel const pxl[static 1], struct SFCoord coord,
                                  double eps);

/** Determine if satellite pixels overlap.
 *
 * Overlapping is defined as one pixel having a vertex / corner that is interior to the other one
 * or as pixels having edges that intersect.
 *
 * The eps parameter is used as a parameter for any cases where floating point values need to be
 * compared. There are a few places in the algorithm where that happens, and if they are within
 * eps units of each other, they are considered equal.
 */
bool satfire_pixels_overlap(struct SFPixel const left[static 1],
                            struct SFPixel const right[static 1], double eps);

/** Determine if satellite pixels are adjacent.
 *
 * Adjacent is defined as having at least one corner that is 'eps' close to a coordinate in the
 * other. Adjacent pixels may overlap also because satfire_pixels_overlap() uses the eps variable in
 * determining overlap. However, if there is a large overlap, the pixels aren't adjacent.
 *
 * \param left a satellite pixel to check.
 * \param right the pixel to check against.
 * \param eps The scale to use for comparison in the same units as the lat and lon.
 **/
bool satfire_pixels_are_adjacent(struct SFPixel const left[static 1],
                                 struct SFPixel const right[static 1], double eps);

/*-------------------------------------------------------------------------------------------------
 *                                         SFPixelList
 *-----------------------------------------------------------------------------------------------*/
/** A pixel list stores a list of SFPixel objects. */
struct SFPixelList {
    size_t len;
    size_t capacity;
    struct SFPixel pixels[];
};

/** Create a new SFPixelList. */
struct SFPixelList *satfire_pixel_list_new();

/** Create a new SFPixelList with a given capacity. */
struct SFPixelList *satfire_pixel_list_new_with_capacity(size_t capacity);

/** Destroy a SFPixelList.  */
struct SFPixelList *satfire_pixel_list_destroy(struct SFPixelList plist[static 1]);

/** Create a deep copy of the SFPixelList. */
struct SFPixelList *satfire_pixel_list_copy(struct SFPixelList const *plist);

/** Append a SFPixel to the list.
 *
 * Reallocates the backing array if necessary and returns the new pointer, so always use the return
 * value as the new list. If the system is running out of memory and the allocation fails, it
 * aborts the program.
 *
 * \return A (potentially new) pointer to the list \param plist.
 */
struct SFPixelList *satfire_pixel_list_append(struct SFPixelList list[static 1],
                                              struct SFPixel const apix[static 1]);

/** Clear the list but keep the memory intact.
 *
 * After this call the list is basically in the same state as after calling
 * satfire_pixel_list_new().
 */
struct SFPixelList *satfire_pixel_list_clear(struct SFPixelList list[static 1]);

/** Calculate the centroid of a SFPixelList. */
struct SFCoord satfire_pixel_list_centroid(struct SFPixelList const list[static 1]);

/** Calculate the total power in a SFPixelList, megawatts. */
double satfire_pixel_list_total_power(struct SFPixelList const list[static 1]);

/** Calculate the total area in a SFPixelList, square meters. */
double satfire_pixel_list_total_area(struct SFPixelList const list[static 1]);

/** Calculate the maximum temperature in a SFPixelList, kelvin. */
double satfire_pixel_list_max_temperature(struct SFPixelList const list[static 1]);

/** Check to see if these two \ref SFPixelList are adjacent or overlapping. */
bool satfire_pixel_lists_adjacent_or_overlap(struct SFPixelList const left[static 1],
                                             struct SFPixelList const right[static 1], double eps);

/*-------------------------------------------------------------------------------------------------
 *                                  Pixel List Binary Format
 *-----------------------------------------------------------------------------------------------*/
// The binary formatting is for storing or retrieving from a database.

/** Calculate the amount of space needed in a buffer to encode a SFPixelList into binary. */
size_t satfire_pixel_list_binary_serialize_buffer_size(struct SFPixelList const plist[static 1]);

/** Encode the SFPixelList into a binary format suitable for storing in a database.
 *
 * At this time it doesn't support a portable format, meaning no corrections are made for
 * endianness or any padding in the array.
 *
 * \return The number of bytes written.
 */
size_t satfire_pixel_list_binary_serialize(struct SFPixelList const plist[static 1],
                                           size_t buf_size, unsigned char buffer[buf_size]);

/** Deserialize an array of bytes into a SFPixelList.
 *
 * \return an allocated SFPixelList that should be cleaned up with satfire_pixel_list_destroy(). In
 * the event of an error, it returns NULL.
 */
struct SFPixelList *
satfire_pixel_list_binary_deserialize(unsigned char const buffer[static sizeof(size_t)]);

/*-------------------------------------------------------------------------------------------------
 *                                         KML Export
 *-----------------------------------------------------------------------------------------------*/
/** Write out a pixel list in KML format.
 *
 * This will print out a multigeometry KML element. It should be composed as part of a function
 * that outputs a KML file where that higher function adds style information and the rest of the
 * document.
 */
void satfire_pixel_list_kml_write(FILE *strm, struct SFPixelList const plist[static 1]);

/*-------------------------------------------------------------------------------------------------
                                                 Cluster
-------------------------------------------------------------------------------------------------*/
/**
 * \struct SFCluster
 * \brief The aggregate properties of a connected group of \ref SFPixel objects.
 */
struct SFCluster;

/** Create a new Cluster. */
struct SFCluster *satfire_cluster_new(void);

/** Cleanup a Cluster. */
void satfire_cluster_destroy(struct SFCluster **cluster);

/** Create a deep copy of a Cluster. */
struct SFCluster *satfire_cluster_copy(struct SFCluster const *cluster);

/** Get the total power of all pixels in the Cluster, megawatts. */
double satfire_cluster_total_power(struct SFCluster const *cluster);

/** Get the total fire area of all pixels in the Cluster that had an area in the file, square
 * meters. */
double satfire_cluster_total_area(struct SFCluster const *cluster);

/** Get the max fire temperature of all pixels in the Cluster that had a temperature in the file,
 * Kelvin. */
double satfire_cluster_max_temperature(struct SFCluster const *cluster);

/** Get the max scan angle of any pixel in this cluster. */
double satfire_cluster_max_scan_angle(struct SFCluster const *cluster);

/** Get the number of SFPixels in a Cluster. */
unsigned int satfire_cluster_pixel_count(struct SFCluster const *cluster);

/** Get access to the pixels in the cluster. */
const struct SFPixelList *satfire_cluster_pixels(struct SFCluster const *cluster);

/** Get the centroid of a cluster. */
struct SFCoord satfire_cluster_centroid(struct SFCluster const *cluster);

/** Compare Cluster objects for sorting in descending order of power. */
int satfire_cluster_descending_power_compare(const void *ap, const void *bp);

/*-------------------------------------------------------------------------------------------------
                                               ClusterList
-------------------------------------------------------------------------------------------------*/
/**
 * \struct SFClusterList
 * \brief Keep a cluster list with metadata about the file it was derived from.
 */
struct SFClusterList;

/**
 * \brief Analyze a file and return a ClusterList.
 *
 * The metadata is gleaned from the file name, so this program relies on the current naming
 * conventions of the NOAA big data program.
 *
 *  \param full_path is the path to the file to analyze.
 */
struct SFClusterList *satfire_cluster_list_from_file(char const *full_path);

/**
 * \brief Clean up a ClusterList object.
 *
 * After this function, the value pointed to by \a list will be set to \c 0 or \c NULL.
 */
void satfire_cluster_list_destroy(struct SFClusterList **list);

/** \brief Get the satellite sector.  */
enum SFSector satfire_cluster_list_sector(struct SFClusterList *list);

/** \brief Get the name of the satellite. */
enum SFSatellite satfire_cluster_list_satellite(struct SFClusterList *list);

/** Get the start time of the scan. */
time_t satfire_cluster_list_scan_start(struct SFClusterList *list);

/** Get the end time of the scan. */
time_t satfire_cluster_list_scan_end(struct SFClusterList *list);

/** Error status from creating the ClusterList.
 *
 * This will always be false unless there was an error creating the ClusterList. In that case the
 * satfire_cluster_list_clusters() function will return \c 0 or \c NULL and the
 * satfire_cluster_list_error_msg() function will return a message as to the source of the error.
 */
bool satfire_cluster_list_error(struct SFClusterList *list);

/** The error message associated with the ClusterList.
 *
 * This is a static string determined at compile time and should not be freed.
 */
const char *satfire_cluster_list_error_msg(struct SFClusterList *list);

/** Get the Clusters.
 *
 * The \c GArray holds pointers to the Cluster objects.
 */
GArray *satfire_cluster_list_clusters(struct SFClusterList *list);

/** \brief Filter the ClusterList to only include fires with their centroid in the BoundingBox.
 *
 * \returns NULL on error or a reference to the same \a list that was passed in.
 */
struct SFClusterList *satfire_cluster_list_filter_box(struct SFClusterList *list,
                                                      struct SFBoundingBox box);

/** \brief Filter the ClusterList to only include fires with their maximum scan angle below a
 * threshold value.
 *
 * \returns NULL on error or a reference to the same \a list that was passed in.
 */
struct SFClusterList *satfire_cluster_list_filter_scan_angle(struct SFClusterList *list,
                                                             double max_scan_angle);

/** \brief Filter the ClusterList to only include fires for which the provided filter function
 * returns \c true.
 *
 * \returns NULL on error or a reference to the same \a list that was passed in. It is important to
 * reassign the provided \a list to the return value of this function in case a reallocation moves
 * the pointer.
 */
struct SFClusterList *satfire_cluster_list_filter(struct SFClusterList *list,
                                                  bool (*filter)(struct SFCluster *clust));

/**
 * \brief Parse the file name and find the scan start time.
 */
char const *satfire_cluster_find_start_time(char const *fname);

/**
 * \brief Parse the file name and find the scan end time.
 */
char const *satfire_cluster_find_end_time(char const *fname);

/**
 * \brief Get the number of items in the ClusterList.
 */
unsigned int satfire_cluster_list_length(struct SFClusterList *list);

/**
 * \brief Get the total fire power of all the clusters in this list.
 */
double satfire_cluster_list_total_power(struct SFClusterList *list);

/*-------------------------------------------------------------------------------------------------
 *                            Query general info about the database
 *-----------------------------------------------------------------------------------------------*/
/** \brief Initialize a database.
 *
 * Initialize a database to make sure it exists and is set up properly. This should be run in the
 * main thread before any other threads open a connection to the database to ensure consistency.
 *
 * \returns 0 on success and non-zero if there is an error.
 */
int satfire_db_initialize(char const *path);

/** A handle to a SFDatabase connection. */
typedef struct SFDatabase *SFDatabaseH;

/**
 * \brief Open a connection to the database to store clusters, wildfires, and associations.
 *
 * \returns the database connect object or NULL if there is an error.
 */
SFDatabaseH satfire_db_connect(char const *path);

/**
 * \brief Close and finalize the connection to the database.
 *
 * The supplied handle will be zeroed out. If the database handle is already NULL, that's OK.
 *
 * \returns 0 if there is no error, otherwise it returns non-zero.
 */
int satfire_db_close(SFDatabaseH *db);

/**
 * \brief Find the latest valid time in the database so you can safely skip anything older.
 *
 * \returns 0 if there is an error or the database is empty, otherwise returns the scan start
 * time of the latest path for that satellite and sector.
 */
time_t satfire_cluster_db_newest_scan_start(SFDatabaseH db, enum SFSatellite satellite,
                                            enum SFSector sector);

/*-------------------------------------------------------------------------------------------------
 *                             Add Rows to the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
/** A handle to an add transaction. */
typedef struct SFClusterDatabaseAdd *SFClusterDatabaseAddH;

/**
 * \brief Prepare to add rows to the cluster database.
 *
 * \returns NULL or the 0 pointer on error.
 */
SFClusterDatabaseAddH satfire_cluster_db_prepare_to_add(SFDatabaseH db);

/**
 * \brief Finalize add transaction.
 *
 * \returns 0 if there is no error.
 */
int satfire_cluster_db_finalize_add(SFClusterDatabaseAddH *stmt);

/**
 * \brief Adds an entire ClusterList to the database.
 *
 * \returns the 0 on success and non-zero on error.
 */
int satfire_cluster_db_add(SFClusterDatabaseAddH stmt, struct SFClusterList *clist);

/*-------------------------------------------------------------------------------------------------
 *                 Query if data from a file is already in the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
/** A handle to a query to check if a file is already in the database. */
typedef struct SFClusterDatabaseQueryPresent *SFClusterDatabaseQueryPresentH;

/**
 * \brief Prepare to query the database if data from a satellite image is already in the database.
 *
 * \return NULL or the 0 pointer on error.
 */
SFClusterDatabaseQueryPresentH satfire_cluster_db_prepare_to_query_present(SFDatabaseH db);

/**
 * \brief Finalize the 'is present' query.
 *
 * \returns 0 if there is no error.
 */
int satfire_cluster_db_finalize_query_present(SFClusterDatabaseQueryPresentH *stmt);

/**
 * \brief Check to see if an entry for these values already exists in the database.
 *
 * \returns the number of items in the database with these values, -1 if there is nothing in the
 * database concerning this satellite, sector, time combination.
 */
int satfire_cluster_db_present(SFClusterDatabaseQueryPresentH query, enum SFSatellite satellite,
                               enum SFSector sector, time_t start, time_t end);

/*-------------------------------------------------------------------------------------------------
 *                            Query rows from the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
/** A handle to a query to get rows from the database. */
typedef struct SFClusterDatabaseQueryRows *SFClusterDatabaseQueryRowsH;

/** Query rows from the database.
 *
 * \param db is a handle to the database file and may not be \c NULL.
 * \param sat - limit results to this satellite only. If this is equal to SATFIRE_SATELLITE_NONE,
 *              then don't limit by satellite.
 * \param sector - limit results to this sector only. If this is equal to SATFIRE_SECTOR_NONE, then
 *                 don't limit by sector.
 * \param start - limit results to ones with a scan start time AFTER this time.
 * \param end - limit results to ones with a scan start time BEFORE this time.
 * \param area - limit results to clusters that have their centroid within the BoundingBox area.
 *
 * \returns a handle to the query object. If there is an error such as a non-existent database, it
 * returns \c NULL.
 */
SFClusterDatabaseQueryRowsH satfire_cluster_db_query_rows(SFDatabaseH db,
                                                          enum SFSatellite const sat,
                                                          enum SFSector const sector,
                                                          time_t const start, time_t const end,
                                                          struct SFBoundingBox const area);

/** \brief Close out and clean up from the query.
 *
 * This will also zero out the handle.
 *
 * \returns 0 if there is no error.
 */
int satfire_cluster_db_query_rows_finalize(SFClusterDatabaseQueryRowsH *query);

/** \struct SFClusterRow
 * \brief A row describing a \ref SFCluster that has been retrieved from the database.
 *
 * A result row from a SFClusterDatabaseQueryRowsH. This includes more information than the
 * \ref SFCluster type does. This also includes valid times and satellite metadata.
 */
struct SFClusterRow;

/** Get the next row.
 *
 * \param query is the handle for the query you want to get the next row on.
 * \param row may be \c NULL, and if so a new row will be allocated. If a non \c NULL pointer is
 *        passed in, then the space it points to will be reused for the next row. If you pass a
 *        pointer in for row, always reassign the result of this function to that variable, as it
 *        may call \c realloc() and move the location of the row in memory.
 *
 * \returns a pointer to the next row, or \c NULL if there is nothing left.
 */
struct SFClusterRow *satfire_cluster_db_query_rows_next(SFClusterDatabaseQueryRowsH query,
                                                        struct SFClusterRow *row);

/** Get the start time of the scan that produced this Cluster. */
time_t satfire_cluster_db_satfire_cluster_row_start(struct SFClusterRow const *row);

/** Get the end time of the scan that produced this Cluster. */
time_t satfire_cluster_db_satfire_cluster_row_end(struct SFClusterRow const *row);

/** Get the fire power in megawatts of this Cluster. */
double satfire_cluster_db_satfire_cluster_row_power(struct SFClusterRow const *row);

/** Get the maximum pixel temperature in Kelvin of this Cluster. */
double satfire_cluster_db_satfire_cluster_row_max_temperature(struct SFClusterRow const *row);

/** Get the fire area in square meters of this Cluster. */
double satfire_cluster_db_satfire_cluster_row_area(struct SFClusterRow const *row);

/** Get the scan angle of this Cluster. */
double satfire_cluster_db_satfire_cluster_row_scan_angle(struct SFClusterRow const *row);

/** Get the centroid of the cluster. */
struct SFCoord satfire_cluster_db_satfire_cluster_row_centroid(struct SFClusterRow const *row);

/** Get the satellite that detected this Cluster. */
enum SFSatellite satfire_cluster_db_satfire_cluster_row_satellite(struct SFClusterRow const *row);

/** Get the scan sector the satellite was using when it detected this Cluster. */
enum SFSector satfire_cluster_db_satfire_cluster_row_sector(struct SFClusterRow const *row);

/** Get view of the SFPixels that make up this Cluster. */
const struct SFPixelList *
satfire_cluster_db_satfire_cluster_row_pixels(struct SFClusterRow const *row);

/** Call this on a SFClusterRow if you're done using it.
 *
 * It's not necessary to call this if you will reuse this SFClusterRow in another call to
 * satfire_cluster_db_query_rows_next(). If \a row is \c NULL, that's also fine.
 *
 * The satfire_cluster_db_query_rows_next() function will deallocate the SFClusterRow object
 * and return \c NULL if there are no more rows, in which case a call to this function is a harmless
 * no-op, but also unnecessary. If you are done using a SFClusterRow object before
 * satfire_cluster_db_query_rows_next() returns \c NULL, then you will need to use this
 * function to clean up the associated allocated memory.
 *
 */
void satfire_cluster_db_satfire_cluster_row_finalize(struct SFClusterRow *row);

/*-------------------------------------------------------------------------------------------------
 *                                        Wildfire
 *-----------------------------------------------------------------------------------------------*/
/**
 * \struct SFWildfire
 * \brief The aggregate properties of a temporally connected group of \ref SFCluster objects.
 */
struct SFWildfire;

/** Create a new wildfire.
 *
 * The \ref SFClusterRow \p initial is left in an invalid state after this function is called. The
 * \ref SFPixelList member pointer is set to \c NULL as creating the new SFWildfire steals the
 * pixels from the \ref SFClusterRow.
 */
struct SFWildfire *satfire_wildfire_new(unsigned int id, struct SFClusterRow *initial);

/** Create a deep copy of this wildfire.
 *
 * If \p source is \c NULL, then \c NULL is returned.
 */
struct SFWildfire *satfire_wildfire_clone(struct SFWildfire const *src);

/** Print out a wildfire to the terminal. */
void satfire_wildfire_print(struct SFWildfire const *src);

/** Cleanup a Wildfire. */
void satfire_wildfire_destroy(struct SFWildfire *wildfire);

/** Get the id number of the fire. */
unsigned int satfire_wildfire_id(struct SFWildfire const *wildfire);

/** Get the time the fire was first observed. */
time_t satfire_wildfire_get_first_observed(struct SFWildfire const *wildfire);

/** Get the time the fire was last observed. */
time_t satfire_wildfire_get_last_observed(struct SFWildfire const *wildfire);

/** Get the time in seconds between the first and last observed times. */
double satfire_wildfire_duration(struct SFWildfire const *wildfire);

/** Get the centroid of a wildfire. */
struct SFCoord satfire_wildfire_centroid(struct SFWildfire const *wildfire);

/** Get the maximum power observed for this fire, megawatts. */
double satfire_wildfire_max_power(struct SFWildfire const *wildfire);

/** Get the max fire temperature observed on this fire, Kelvin. */
double satfire_wildfire_max_temperature(struct SFWildfire const *wildfire);

/** Get access to the pixels in the wildfire. */
struct SFPixelList const *satfire_wildfire_pixels(struct SFWildfire const *wildfire);

/** Get the satellite this fire was observed from. */
enum SFSatellite satfire_wildfire_satellite(struct SFWildfire const *wildfire);

/** Update a wildfire by adding the information in this \ref SFClusterRow to it. */
void satfire_wildfire_update(struct SFWildfire *wildfire, struct SFClusterRow const *row);

/*-------------------------------------------------------------------------------------------------
 *                                        Wildfire List
 *-----------------------------------------------------------------------------------------------*/
/**
 * \struct SFWildfireList
 * \brief A list of wildfires.
 */
struct SFWildfireList;

/** Clean up the memory associated with this \ref SFWildfireList.
 *
 * \returns the updated pointer to the list, in this case it should be NULL.
 */
struct SFWildfireList *satfire_wildfirelist_destroy(struct SFWildfireList *list);

/** Add a wildfire to the list.
 *
 * The pointer to the list may be reallocated, so the argument \p list should be assigned the return
 * value. This ensures that it is not left dangling.
 *
 * \param list is the list to add the new fire to. If this is \c NULL, then a new list is created.
 * \param new_fire is the fire to be added to the \p list, the \p list takes ownership of the fire.
 *
 * \returns a pointer to the (possibly new) location of \p list.
 */
struct SFWildfireList *satfire_wildfirelist_add_fire(struct SFWildfireList *list,
                                                     struct SFWildfire *new_fire);

/** Create a new wildfire and add it to the list.
 *
 * The pointer to the list may be reallocated, so the argument \p list should be assigned the return
 * value. This ensures that it is not left dangling.
 *
 * The \ref SFClusterRow \p initial is left in an invalid state after this function is called. The
 * \ref SFPixelList member pointer is set to \c NULL as creating the new SFWildfire steals the
 * pixels from the \ref SFClusterRow.
 *
 * \param list is the list to add the new fire to. If this is \c NULL, then a new list is created.
 * \param id is the id number to be forwarded to satfire_wildfire_new().
 * \param initial is the initial \ref SFClusterRow to be forwarded to satfire_wildfire_new().
 *
 * \returns a pointer to the (possibly new) location of \p list.
 *
 * \see satfire_wildfire_new()
 */
struct SFWildfireList *satfire_wildfirelist_create_add_fire(struct SFWildfireList *list,
                                                            unsigned int id,
                                                            struct SFClusterRow *initial);

/** Update the list with the provided cluster.
 *
 * Matches the cluster to a wildfire in the list and then updates that wildfire.
 *
 * \param list is the list to search and see if you can find a wildfire that matches this cluster.
 * \param clust is the cluster you are trying to assign to the fire.
 *
 * \returns \c true if \p clust was matched to a wildfire and used to update it, returns \c false
 * otherwise.
 */
bool satfire_wildfirelist_update(struct SFWildfireList *const list,
                                 struct SFClusterRow const *clust);

/** Extend a wildfire list using another wildfire list.
 *
 * Modifies \p list by moving the elements of \p src to it. The parameter \p list should have the
 * return value assigned back to it in case there was a reallocation, and \p src will be left empty
 * but with all of it's memory still allocated. So when you're finally done with it you'll need to
 * call \ref satfire_wildfirelist_destroy() on it.
 */
struct SFWildfireList *satfire_wildfirelist_extend(struct SFWildfireList *list,
                                                   struct SFWildfireList *const src);

/** Detect overlaps in the wildfires in the list and merge them together into a single fire.
 *
 * Fires that are merged into another fire, and so they no longer exist are moved to the
 * \p merged_away list. The return value of this list should be assigned to the \p merged_away list
 * in case a reallocation occurred and the pointer moved.
 *
 * \param list is the list of wildfires to be checked for mergers.
 * \param merged_away is a list that will be grown with the fires that are removed because they were
 * merged into another fire. This pointer may be \c NULL if you want to start a new list.
 *
 * \returns the updated location of the \p merged_away list.
 */
struct SFWildfireList *satfire_wildfirelist_merge_fires(struct SFWildfireList *const list,
                                                        struct SFWildfireList *merged_away);

/** Remove fires older than \p older_than from the list and place them in \p tgt_list.
 *
 * \param list is the source list to drain fires from if they are older than \p older_than.
 * \param tgt_list is the list to add the drained elements into. If this point is \c NULL, then a
 * new list will be created. The return value of this function should be assigned to the variable
 * that was passed into this argument in case it was moved for a reallocation.
 * \param older_than, all fires last observed than this time will be moved to the \p tgt_list.
 *
 * \returns an updated pointer to \p tgt_list.
 */
struct SFWildfireList *
satfire_wildfirelist_drain_fires_not_seen_since(struct SFWildfireList *const list,
                                                struct SFWildfireList *tgt_list, time_t older_than);

/** Get the number of fires in the list. */
size_t satfire_wildfirelist_len(struct SFWildfireList const *list);

/** Get a reference to an element at a given index. */
struct SFWildfire const *satfire_wildfirelist_get(struct SFWildfireList const *list, size_t index);

/*-------------------------------------------------------------------------------------------------
 *                             Wildfire Database Query Metadata
 *-----------------------------------------------------------------------------------------------*/
/**
 * \brief Get the next id number for a wildfire.
 */
unsigned int satfire_fires_db_next_wildfire_id(SFDatabaseH db);

/*-------------------------------------------------------------------------------------------------
 *                             Add Rows to the Fires Database
 *-----------------------------------------------------------------------------------------------*/
/** A handle to an add transaction. */
typedef struct SFFiresDatabaseAdd *SFFiresDatabaseAddH;

/**
 * \brief Prepare to add rows to the fires database.
 *
 * \returns NULL or the 0 pointer on error.
 */
SFFiresDatabaseAddH satfire_fires_db_prepare_to_add(SFDatabaseH db);

/**
 * \brief Finalize add transaction.
 *
 * \returns 0 if there is no error.
 */
int satfire_fires_db_finalize_add(SFFiresDatabaseAddH *stmt);

/**
 * \brief Adds or updates a fire to the database.
 *
 * \returns the 0 on success and non-zero on error.
 */
int satfire_fires_db_add(SFFiresDatabaseAddH stmt, struct SFWildfire *fire);
