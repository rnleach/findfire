#include "database.h"

#include <assert.h>
#include <stdbool.h>

#include "util.h"

sqlite3 *
cluster_db_connect(char const *path)
{
    sqlite3 *handle = 0;
    int rc = sqlite3_open_v2(path, &handle, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error connecting to %s", path);

    char *query = "CREATE TABLE IF NOT EXISTS clusters(\n"
                  "satellite  TEXT    NOT NULL,        \n"
                  "sector     TEXT    NOT NULL,        \n"
                  "start_time INTEGER NOT NULL,        \n"
                  "lat        REAL    NOT NULL,        \n"
                  "lon        REAL    NOT NULL,        \n"
                  "power      REAL    NOT NULL,        \n"
                  "cell_count INTEGER NOT NULL)        \n";
    char *err_message = 0;

    rc = sqlite3_exec(handle, query, 0, 0, &err_message);
    if (rc != SQLITE_OK) {
        printf("Error initializing database: %s\n\n", err_message);
        sqlite3_free(err_message);
        goto ERR_CLEANUP;
    }

    return handle;

ERR_CLEANUP:
    sqlite3_close(handle);
    return 0;
}

int
cluster_db_close(sqlite3 **db)
{
    int rc = sqlite3_close(*db);
    *db = 0;
    return rc;
}

sqlite3_stmt *
cluster_db_prepare_to_add(sqlite3 *db)
{
    char *err_message = 0;

    char *query = "BEGIN TRANSACTION";
    int rc = sqlite3_exec(db, query, 0, 0, &err_message);
    if (rc != SQLITE_OK) {
        printf("Error starting transaction: %s\n\n", err_message);
        sqlite3_free(err_message);
        return 0;
    }

    query = "INSERT OR REPLACE INTO clusters (                      \n"
            "satellite, sector, start_time, lat, lon, power, cell_count \n"
            ") VALUES (?, ?, ?, ?, ?, ?, ?)";

    sqlite3_stmt *stmt = 0;
    rc = sqlite3_prepare_v2(db, query, -1, &stmt, 0);
    Stopif(rc != SQLITE_OK, return stmt, "Error preparing statement: %s", sqlite3_errstr(rc));

    return stmt;
}

int
cluster_db_finalize_add(sqlite3 *db, sqlite3_stmt **stmt)
{
    char *err_message = 0;

    char *query = "COMMIT TRANSACTION";
    int rc = sqlite3_exec(db, query, 0, 0, &err_message);
    if (rc != SQLITE_OK) {
        printf("Error commiting transaction: %s\n\n", err_message);
        sqlite3_free(err_message);
        return 0;
    }

    rc = sqlite3_finalize(*stmt);
    *stmt = 0;
    return rc;
}

int
cluster_db_add_row(sqlite3_stmt *stmt, char const *satellite, char const *sector, time_t scan_start,
                   float lat, float lon, float power, int num_points)
{
    int rc = sqlite3_bind_text(stmt, 1, satellite, -1, 0);
    Stopif(rc != SQLITE_OK, return 1, "Error binding satellite: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_text(stmt, 2, sector, -1, 0);
    Stopif(rc != SQLITE_OK, return 1, "Error binding sector: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_int64(stmt, 3, scan_start);
    Stopif(rc != SQLITE_OK, return 1, "Error binding start time: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_double(stmt, 4, lat);
    Stopif(rc != SQLITE_OK, return 1, "Error binding lat: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_double(stmt, 5, lon);
    Stopif(rc != SQLITE_OK, return 1, "Error binding lon: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_double(stmt, 6, power);
    Stopif(rc != SQLITE_OK, return 1, "Error binding power: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_int(stmt, 7, num_points);
    Stopif(rc != SQLITE_OK, return 1, "Error binding cell count: %s", sqlite3_errstr(rc));

    rc = sqlite3_step(stmt);
    Stopif(rc != SQLITE_OK && rc != SQLITE_DONE, return 1, "Error stepping: %s",
           sqlite3_errstr(rc));

    rc = sqlite3_reset(stmt);
    Stopif(rc != SQLITE_OK, return 1, "Error resetting: %s", sqlite3_errstr(rc));

    return 0;
}