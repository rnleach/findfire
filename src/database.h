#pragma once

#include <time.h>

#include "sqlite3.h"

/**
 * \brief Open a connection to the database to store clusters.
 *
 * \returns the database connect object or NULL if there is an error.
 */
sqlite3 *cluster_db_connect(char const *path);

/**
 * \brief Close and finalize the connection to the database.
 *
 * The supplied pointer will be zeroed out.
 *
 * \returns SQLITE_OK if there is no error, otherwise it returns an sqlite3 error code.
 */
int cluster_db_close(sqlite3 **db);

/**
 * \brief Prepare to add rows to the cluster database.
 *
 * \returns NULL or the 0 pointer on error.
 */
sqlite3_stmt *cluster_db_prepare_to_add(sqlite3 *db);

/**
 * \brief Finalize add and commit data to the database.
 *
 * \returns SQLITE_OK if there is no error.
 */
int cluster_db_finalize_add(sqlite3 *db, sqlite3_stmt **stmt);

/**
 * \brief Binds values and steps through adding the values to the database.
 *
 * \returns the 0 on success and non-zero on error.
 */
int cluster_db_add_row(sqlite3_stmt *stmt, char const *satellite, char const *sector,
                       time_t scan_start, float lat, float lon, float power, float radius,
                       int num_points);
