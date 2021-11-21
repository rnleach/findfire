#pragma once

#include <time.h>

#include "cluster.h"
#include "geo.h"
#include "satellite.h"

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
int cluster_db_initialize(char const *path);

/** A handle to a ClusterDatabase connection. */
typedef struct ClusterDatabase *ClusterDatabaseH;

/**
 * \brief Open a connection to the database to store clusters.
 *
 * \returns the database connect object or NULL if there is an error.
 */
ClusterDatabaseH cluster_db_connect(char const *path);

/**
 * \brief Close and finalize the connection to the database.
 *
 * The supplied handle will be zeroed out. If the database handle is already NULL, that's OK.
 *
 * \returns 0 if there is no error, otherwise it returns non-zero.
 */
int cluster_db_close(ClusterDatabaseH *db);

/**
 * \brief Find the latest valid time in the database so you can safely skip anything older.
 *
 * \returns 0 if there is an error or the database is empty, otherwise returns the scan start
 * time of the latest path for that satellite and sector.
 */
time_t cluster_db_newest_scan_start(ClusterDatabaseH db, enum Satellite satellite,
                                    enum Sector sector);

/*-------------------------------------------------------------------------------------------------
 *                             Add Rows to the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
/** A handle to an add transaction. */
typedef struct ClusterDatabaseAdd *ClusterDatabaseAddH;

/**
 * \brief Prepare to add rows to the cluster database.
 *
 * \returns NULL or the 0 pointer on error.
 */
ClusterDatabaseAddH cluster_db_prepare_to_add(char const *path_to_db);

/**
 * \brief Finalize add transaction.
 *
 * \returns 0 if there is no error.
 */
int cluster_db_finalize_add(ClusterDatabaseAddH *stmt);

/**
 * \brief Adds an entire ClusterList to the database.
 *
 * \returns the 0 on success and non-zero on error.
 */
int cluster_db_add(ClusterDatabaseAddH stmt, struct ClusterList *clist);

/*-------------------------------------------------------------------------------------------------
 *                 Query if data from a file is already in the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
/** A handle to a query to check if a file is already in the database. */
typedef struct ClusterDatabaseQueryPresent *ClusterDatabaseQueryPresentH;

/**
 * \brief Prepare to query the database if data from a satellite image is already in the database.
 *
 * \return NULL or the 0 pointer on error.
 */
ClusterDatabaseQueryPresentH cluster_database_prepare_to_query_present(char const *path_to_db);

/**
 * \brief Finalize the 'is present' query.
 *
 * \returns 0 if there is no error.
 */
int cluster_db_finalize_query_present(ClusterDatabaseQueryPresentH *stmt);

/**
 * \brief Check to see if an entry for these values already exists in the database.
 *
 * \returns the number of items in the database with these values, -1 if there is nothing in the
 * database concerning this satellite, sector, time combination.
 */
int cluster_db_present(ClusterDatabaseQueryPresentH query, enum Satellite satellite,
                       enum Sector sector, time_t start, time_t end);

/*-------------------------------------------------------------------------------------------------
 *                            Query rows from the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
/** A handle to a query to get rows from the database. */
typedef struct ClusterDatabaseQueryRows *ClusterDatabaseQueryRowsH;

/** Query rows from the database.
 *
 * \param path_to_db is the location of the database file and may not be \c NULL.
 * \param sat - limit results to this satellite only. If this is \c NULL or equal to
 *        SATFIRE_SATELLITE_NONE, then don't limit by satellite.
 * \param sector - limit results to this sector only. If this is \c NULL or equal to
 *        SATFIRE_SECTOR_NONE, then don't limit by sector.
 * \param start - limit results to ones with a scan start time AFTER this time. If this is \c NULL,
 *        then don't place a minimum start time limit on the results.
 * \param end - limit results to ones with a scan start time BEFORE this time. If this is \c NULL,
 *        then don't place a maximum start time limit on the results.
 * \param area - limit results to clusters that have their centroid within the BoundingBox area. If
 *        this is \c NULL, then don't place a geographic limit on the results.
 *
 * \returns a handle to the query object. If there is an error such as a non-existent database, it
 * returns \c NULL.
 */
ClusterDatabaseQueryRowsH cluster_db_query_rows(char const *path_to_db, enum Satellite const *sat,
                                                enum Sector const *sector, time_t const *start,
                                                time_t const *end, struct BoundingBox const *area);

/** \brief Close out and clean up from the query.
 *
 * This will also zero out the handle.
 *
 * \returns 0 if there is no error.
 */
int cluster_db_query_rows_finalize(ClusterDatabaseQueryRowsH *query);

/** A result row from a ClusterDatabaseQueryRowsH. */
struct ClusterRow;

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
struct ClusterRow *cluster_db_query_rows_next(ClusterDatabaseQueryRowsH query,
                                              struct ClusterRow *row);

/** Get the start time of the scan that produced this Cluster. */
time_t cluster_db_cluster_row_start(struct ClusterRow *row);

/** Get the end time of the scan that produced this Cluster. */
time_t cluster_db_cluster_row_end(struct ClusterRow *row);

/** Get the fire power in megawatts of this Cluster. */
double cluster_db_cluster_row_power(struct ClusterRow *row);

/** Get the scan angle of this Cluster. */
double cluster_db_cluster_row_scan_angle(struct ClusterRow *row);

/** Get the satellite that detected this Cluster. */
enum Satellite cluster_db_cluster_row_satellite(struct ClusterRow *row);

/** Get the scan sector the satellite was using when it detected this Cluster. */
enum Sector cluster_db_cluster_row_sector(struct ClusterRow *row);

/** Get view of the SatPixels that make up this Cluster. */
const struct PixelList *cluster_db_cluster_row_pixels(struct ClusterRow *row);

/** Call this on a ClusterRow if you're done using it.
 *
 * It's not necessary to call this if you will reuse this ClusterRow in another call to
 * cluster_db_query_rows_next(). If \a row is \c NULL, that's also fine.
 *
 * The cluster_db_query_rows_next() function will deallocate the ClusterRow object and return
 * \c NULL if there are no more rows, in which case a call to this function is a harmless no-op, but
 * also unnecessary. If you are done using a ClusterRow object before cluster_db_query_rows_next()
 * returns \c NULL, then you will need to use this function to clean up the associated allocated
 * memory.
 *
 */
void cluster_db_cluster_row_finalize(struct ClusterRow *row);
