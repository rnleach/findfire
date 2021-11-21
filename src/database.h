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
