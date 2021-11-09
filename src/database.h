#pragma once

#include <time.h>

#include "cluster.h"
#include "geo.h"
#include "satellite.h"

/** A handle to a ClusterDatabase connection. */
typedef struct ClusterDatabase *ClusterDatabaseH;

/** A handle to an add transaction. */
typedef struct ClusterDatabaseAdd *ClusterDatabaseAddH;

/** A handle to a query to check if a file is already in the database. */
typedef struct ClusterDatabaseQueryPresent *ClusterDatabaseQueryPresentH;

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
 * \brief Prepare to add rows to the cluster database.
 *
 * \returns NULL or the 0 pointer on error.
 */
ClusterDatabaseAddH cluster_db_prepare_to_add(ClusterDatabaseH db);

/**
 * \brief Finalize add transaction.
 *
 * \returns 0 if there is no error.
 */
int cluster_db_finalize_add(ClusterDatabaseH db, ClusterDatabaseAddH *stmt);

/**
 * \brief Adds an entire ClusterList to the database.
 *
 * \returns the 0 on success and non-zero on error.
 */
int cluster_db_add(ClusterDatabaseH db, ClusterDatabaseAddH stmt, struct ClusterList *clist);

/**
 * \brief Find the latest valid time in the database so you can safely skip anything older.
 *
 * \returns 0 if there is an error or the database is empty, otherwise returns the scan start
 * time of the latest path for that satellite and sector.
 */
time_t cluster_db_newest_scan_start(ClusterDatabaseH db, enum Satellite satellite,
                                    char const *sector);

/**
 * \brief Prepare to query the database if data from a satellite image is already in the database.
 *
 * \return NULL or the 0 pointer on error.
 */
ClusterDatabaseQueryPresentH cluster_database_prepare_to_query_present(ClusterDatabaseH db);

/**
 * \brief Finalize the 'is present' query.
 *
 * \returns 0 if there is no error.
 */
int cluster_db_finalize_query_present(ClusterDatabaseH db, ClusterDatabaseQueryPresentH *stmt);

/**
 * \brief Check to see if an entry for these values already exists in the database.
 *
 * \returns the number of items in the database with these values, -1 if there is nothing in the
 * database concerning this satellite, sector, time combination.
 */
int cluster_db_present(ClusterDatabaseQueryPresentH query, enum Satellite satellite,
                       char const *sector, time_t start, time_t end);
