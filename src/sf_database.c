#include "satfire.h"

#include <assert.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#include <sqlite3.h>

#include "sf_util.h"

/*-------------------------------------------------------------------------------------------------
 *                               Open & Close the database
 *-----------------------------------------------------------------------------------------------*/
static int
open_database_to_write(char const *path, sqlite3 **result)
{
    static_assert(SQLITE_OK == 0, "SQLITE_OK must equal 0 or we'll have problems here.");
    assert(result);

    sqlite3 *handle = 0;
    int rc = sqlite3_open_v2(path, &handle,
                             SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_NOMUTEX, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error connecting to %s", path);

    // A 5-second busy time out is WAY too much. If we hit this something has gone terribly wrong.
    sqlite3_busy_timeout(handle, 5000);

    char *query = "CREATE TABLE IF NOT EXISTS clusters (                 \n"
                  "  satellite      TEXT    NOT NULL,                    \n"
                  "  sector         TEXT    NOT NULL,                    \n"
                  "  start_time     INTEGER NOT NULL,                    \n"
                  "  end_time       INTEGER NOT NULL,                    \n"
                  "  lat            REAL    NOT NULL,                    \n"
                  "  lon            REAL    NOT NULL,                    \n"
                  "  power          REAL    NOT NULL,                    \n"
                  "  max_scan_angle REAL    NOT NULL,                    \n"
                  "  pixels         BLOB    NOT NULL);                   \n"
                  "                                                      \n"
                  "CREATE UNIQUE INDEX IF NOT EXISTS no_cluster_dups     \n"
                  "  ON clusters (satellite, sector, start_time,         \n"
                  "               end_time, lat, lon);                   \n"
                  "                                                      \n"
                  "CREATE INDEX IF NOT EXISTS file_processed             \n"
                  "  ON clusters (satellite, sector, start_time,         \n"
                  "               end_time);                             \n"
                  "                                                      \n"
                  "CREATE TABLE IF NOT EXISTS no_fire (                  \n"
                  "  satellite  TEXT    NOT NULL,                        \n"
                  "  sector     TEXT    NOT NULL,                        \n"
                  "  start_time INTEGER NOT NULL,                        \n"
                  "  end_time   INTEGER NOT NULL);                       \n";

    char *err_message = 0;

    rc = sqlite3_exec(handle, query, 0, 0, &err_message);
    if (rc != SQLITE_OK) {
        fprintf(stderr, "Error initializing database: %s\n\n", err_message);
        sqlite3_free(err_message);
        goto ERR_CLEANUP;
    }

    *result = handle;

    return rc;

ERR_CLEANUP:
    sqlite3_close(handle);
    return rc;
}

static sqlite3 *
open_database_readonly(char const *path)
{
    //
    // The mode flag should be SQLITE_OPEN_READONLY instead of SQLITE_OPEN_READWRITE, however,
    // when accessing a database from my mac this with multiple threads (some read, some write)
    // this causes an SQLITE disk I/O error. It works fine on Ubuntu/PoP!_OS. But I'll leave it
    // this way for now so it works on both machines!
    //
    sqlite3 *handle = 0;
    int rc = sqlite3_open_v2(path, &handle, SQLITE_OPEN_READWRITE | SQLITE_OPEN_NOMUTEX, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error connecting to %s", path);

    // A 5-second busy time out is WAY too much. If we hit this something has gone terribly wrong.
    sqlite3_busy_timeout(handle, 5000);

    return handle;

ERR_CLEANUP:
    sqlite3_close(handle);
    return 0;
}

static int
close_database(sqlite3 **db)
{
    static_assert(SQLITE_OK == 0, "SQLITE_OK must equal 0 or we'll have problems here.");

    assert(db);

    if (*db) {
        int rc = sqlite3_close(*db);
        *db = 0;
        return rc;
    }

    return 0;
}

/*-------------------------------------------------------------------------------------------------
 *                            Query general info about the database
 *-----------------------------------------------------------------------------------------------*/
int
satfire_cluster_db_initialize(char const *path)
{
    sqlite3 *db = 0;

    int rc = open_database_to_write(path, &db);
    Stopif(rc != SQLITE_OK, return rc, "Error initializing the database: %s", sqlite3_errstr(rc));

    rc = close_database(&db);
    Stopif(rc != SQLITE_OK, return rc, "Error initializing the database: %s", sqlite3_errstr(rc));

    return rc;
}

struct SFClusterDatabase {
    sqlite3 *ptr;
};

struct SFClusterDatabase *
satfire_cluster_db_connect(char const *path)
{
    sqlite3 *handle = open_database_readonly(path);

    struct SFClusterDatabase *cdbh = malloc(sizeof(struct SFClusterDatabase));
    Stopif(!cdbh, goto ERR_CLEANUP, "out of memory");
    cdbh->ptr = handle;

    return cdbh;

ERR_CLEANUP:

    sqlite3_close(handle);
    return 0;
}

int
satfire_cluster_db_close(struct SFClusterDatabase **db)
{
    assert(db);

    if (*db) {
        int rc = close_database(&(*db)->ptr);
        free(*db);
        *db = 0;
        return rc;
    }

    return 0;
}

time_t
satfire_cluster_db_newest_scan_start(struct SFClusterDatabase *db, enum SFSatellite satellite,
                                     enum SFSector sector)
{
    int rc = SQLITE_OK;
    time_t newest_scan_time = 0;
    char *query = 0;
    sqlite3_stmt *stmt = 0;

    if (db->ptr) {
        asprintf(&query,
                 "SELECT MAX(start_time) FROM clusters WHERE satellite = '%s' AND sector = '%s'",
                 satfire_satellite_name(satellite), satfire_sector_name(sector));

        rc = sqlite3_prepare_v2(db->ptr, query, -1, &stmt, 0);
        Stopif(rc != SQLITE_OK, goto CLEAN_UP, "Error preparing newest scan statement: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_step(stmt);
        Stopif(rc != SQLITE_ROW, goto CLEAN_UP, "Error stepping: %s", sqlite3_errstr(rc));

        // Check for NULL
        if (sqlite3_column_type(stmt, 0) != SQLITE_INTEGER) {
            goto CLEAN_UP;
        }

        newest_scan_time = sqlite3_column_int64(stmt, 0);
    }

CLEAN_UP:
    free(query);
    rc = sqlite3_finalize(stmt);
    Stopif(rc != SQLITE_OK, return newest_scan_time, "Error finalizing: %s", sqlite3_errstr(rc));

    return newest_scan_time;
}

/*-------------------------------------------------------------------------------------------------
 *                             Add Rows to the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
struct SFClusterDatabaseAdd {
    sqlite3 *db;
    sqlite3_stmt *add_ptr;
    sqlite3_stmt *no_fire_ptr;
};

struct SFClusterDatabaseAdd *
satfire_cluster_db_prepare_to_add(char const *path_to_db)
{
    assert(path_to_db);

    struct SFClusterDatabaseAdd *add = 0;
    sqlite3 *db = 0;
    sqlite3_stmt *add_stmt = 0;
    sqlite3_stmt *no_fire_stmt = 0;

    int rc = open_database_to_write(path_to_db, &db);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "unable to open a connection to the database: %s",
           path_to_db);

    char *add_query =
        "INSERT OR REPLACE INTO clusters (                                                  \n"
        "  satellite, sector, start_time, end_time, lat, lon, power, max_scan_angle, pixels)\n"
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)                                                 \n";

    rc = sqlite3_prepare_v2(db, add_query, -1, &add_stmt, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing statement: %s", sqlite3_errstr(rc));

    char *no_fire_query = "INSERT OR REPLACE INTO no_fire              \n"
                          "  (satellite, sector, start_time, end_time) \n"
                          "VALUES (?, ?, ?, ?)                         \n";

    rc = sqlite3_prepare_v2(db, no_fire_query, -1, &no_fire_stmt, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing statement: %s", sqlite3_errstr(rc));

    add = malloc(sizeof(struct SFClusterDatabaseAdd));
    Stopif(!add, goto ERR_CLEANUP, "out of memory");

    add->db = db;
    add->add_ptr = add_stmt;
    add->no_fire_ptr = no_fire_stmt;

    return add;

ERR_CLEANUP:
    free(add);
    sqlite3_finalize(add_stmt);
    sqlite3_finalize(no_fire_stmt);
    close_database(&db);

    return 0;
}

int
satfire_cluster_db_finalize_add(struct SFClusterDatabaseAdd **stmt)
{
    static_assert(SQLITE_OK == 0, "SQLITE_OK must equal 0 or we'll have problems here.");

    assert(stmt && (*stmt) && (*stmt)->add_ptr && (*stmt)->no_fire_ptr && (*stmt)->db);

    int rc = SQLITE_OK;
    int rc_x = sqlite3_finalize((*stmt)->add_ptr);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error finalizing add statement: %s",
           sqlite3_errstr(rc_x));

    rc_x = sqlite3_finalize((*stmt)->no_fire_ptr);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error finalizing no fire statement: %s",
           sqlite3_errstr(rc_x));

    rc_x = close_database(&(*stmt)->db);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error closing database connection: %s",
           sqlite3_errstr(rc_x));

    free(*stmt);
    *stmt = 0;

    return rc;
}

static int
satfire_cluster_db_add_cluster(struct SFClusterDatabaseAdd *stmt, struct SFClusterList *clist)
{
    assert(stmt && stmt->add_ptr && stmt->db && clist);

    int rc = SQLITE_OK;
    char *err_message = 0;

    char *begin_trans = "BEGIN TRANSACTION";
    rc = sqlite3_exec(stmt->db, begin_trans, 0, 0, &err_message);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error starting transaction: %s", err_message);

    enum SFSatellite satellite = satfire_cluster_list_satellite(clist);
    enum SFSector sector = satfire_cluster_list_sector(clist);
    time_t scan_start = satfire_cluster_list_scan_start(clist);
    time_t scan_end = satfire_cluster_list_scan_end(clist);

    GArray *clusters = satfire_cluster_list_clusters(clist);

    unsigned char buffer[1024] = {0};

    for (unsigned int i = 0; i < clusters->len; ++i) {

        struct SFCluster *cluster = g_array_index(clusters, struct SFCluster *, i);

        rc = sqlite3_bind_text(stmt->add_ptr, 1, satfire_satellite_name(satellite), -1, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding satellite: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_bind_text(stmt->add_ptr, 2, satfire_sector_name(sector), -1, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding sector: %s", sqlite3_errstr(rc));

        rc = sqlite3_bind_int64(stmt->add_ptr, 3, scan_start);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_bind_int64(stmt->add_ptr, 4, scan_end);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s",
               sqlite3_errstr(rc));

        struct SFCoord centroid = satfire_cluster_centroid(cluster);

        rc = sqlite3_bind_double(stmt->add_ptr, 5, centroid.lat);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding lat: %s", sqlite3_errstr(rc));

        rc = sqlite3_bind_double(stmt->add_ptr, 6, centroid.lon);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding lon: %s", sqlite3_errstr(rc));

        rc = sqlite3_bind_double(stmt->add_ptr, 7, satfire_cluster_total_power(cluster));
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding power: %s", sqlite3_errstr(rc));

        rc = sqlite3_bind_double(stmt->add_ptr, 8, satfire_cluster_max_scan_angle(cluster));
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding max scan angle: %s",
               sqlite3_errstr(rc));

        unsigned char *buf_ptr = buffer;
        void (*transient_free)(void *) = SQLITE_TRANSIENT;
        size_t buff_size =
            satfire_pixel_list_binary_serialize_buffer_size(satfire_cluster_pixels(cluster));
        if (buff_size > sizeof(buffer)) {
            transient_free = free; // free function from stdlib.h
            buf_ptr = calloc(buff_size, sizeof(unsigned char));
            Stopif(!buf_ptr, exit(EXIT_FAILURE), "calloc failure: out of memory");
        }

        size_t num_bytes_serialized = satfire_pixel_list_binary_serialize(
            satfire_cluster_pixels(cluster), buff_size, buf_ptr);
        Stopif(num_bytes_serialized != buff_size, exit(EXIT_FAILURE),
               "Buffer size error serializing PixelList");
        rc = sqlite3_bind_blob(stmt->add_ptr, 9, buf_ptr, buff_size, transient_free);

        rc = sqlite3_step(stmt->add_ptr);
        Stopif(rc != SQLITE_OK && rc != SQLITE_DONE, goto ERR_CLEANUP,
               "Error stepping: %s (%s, %u)", sqlite3_errstr(rc), __FILE__, __LINE__);

        rc = sqlite3_reset(stmt->add_ptr);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error resetting: %s", sqlite3_errstr(rc));
    }

    char *commit_trans = "COMMIT TRANSACTION";
    rc = sqlite3_exec(stmt->db, commit_trans, 0, 0, &err_message);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error committing transaction: %s", err_message);

    return 0;

ERR_CLEANUP:

    rc = sqlite3_exec(stmt->db, "ROLLBACK TRANSACTION", 0, 0, &err_message);
    if (rc != SQLITE_OK) {
        fprintf(stderr, "Error rolling back failed transaction: %s\n", sqlite3_errstr(rc));
    }

    sqlite3_reset(stmt->add_ptr);
    sqlite3_free(err_message);
    return 1;
}

static int
satfire_cluster_db_add_no_fire(struct SFClusterDatabaseAdd *stmt, struct SFClusterList *clist)
{
    assert(stmt && stmt->no_fire_ptr && clist);

    int rc = SQLITE_OK;
    char *err_message = 0;

    enum SFSatellite satellite = satfire_cluster_list_satellite(clist);
    enum SFSector sector = satfire_cluster_list_sector(clist);
    time_t scan_start = satfire_cluster_list_scan_start(clist);
    time_t scan_end = satfire_cluster_list_scan_end(clist);

    rc = sqlite3_bind_text(stmt->no_fire_ptr, 1, satfire_satellite_name(satellite), -1, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding satellite: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_text(stmt->no_fire_ptr, 2, satfire_sector_name(sector), -1, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding sector: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_int64(stmt->no_fire_ptr, 3, scan_start);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_int64(stmt->no_fire_ptr, 4, scan_end);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s", sqlite3_errstr(rc));

    rc = sqlite3_step(stmt->no_fire_ptr);
    Stopif(rc != SQLITE_OK && rc != SQLITE_DONE, goto ERR_CLEANUP, "Error stepping: %s",
           sqlite3_errstr(rc));

    rc = sqlite3_reset(stmt->no_fire_ptr);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error resetting: %s", sqlite3_errstr(rc));

    return 0;

ERR_CLEANUP:

    sqlite3_reset(stmt->no_fire_ptr);
    sqlite3_free(err_message);
    return 1;
}

int
satfire_cluster_db_add(struct SFClusterDatabaseAdd *stmt, struct SFClusterList *clist)
{
    GArray *clusters = satfire_cluster_list_clusters(clist);
    if (clusters->len > 0) {
        return satfire_cluster_db_add_cluster(stmt, clist);
    } else {
        return satfire_cluster_db_add_no_fire(stmt, clist);
    }
}

/*-------------------------------------------------------------------------------------------------
 *                 Query if data from a file is already in the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
struct SFClusterDatabaseQueryPresent {
    sqlite3 *db;
    sqlite3_stmt *count_stmt;
    sqlite3_stmt *no_fire_stmt;
};

struct SFClusterDatabaseQueryPresent *
satfire_cluster_db_prepare_to_query_present(char const *path_to_db)
{
    int rc = SQLITE_OK;
    struct SFClusterDatabaseQueryPresent *query = 0;
    sqlite3 *db = 0;
    sqlite3_stmt *stmt_clusters = 0;
    sqlite3_stmt *stmt_no_fire = 0;

    db = open_database_readonly(path_to_db);

    if (db) {
        char *query_clusters =
            "SELECT COUNT(*) FROM clusters                                         \n"
            "WHERE satellite = ? AND sector = ? AND start_time = ? AND end_time = ?\n";

        rc = sqlite3_prepare_v2(db, query_clusters, -1, &stmt_clusters, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing count rows statement: %s",
               sqlite3_errstr(rc));

        char *query_no_fire =
            "SELECT COUNT(*) FROM no_fire                                          \n"
            "WHERE satellite = ? AND sector = ? AND start_time = ? AND end_time = ?\n";

        rc = sqlite3_prepare_v2(db, query_no_fire, -1, &stmt_no_fire, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing count rows statement: %s",
               sqlite3_errstr(rc));

        query = malloc(sizeof(struct SFClusterDatabaseQueryPresent));
        Stopif(!query, goto ERR_CLEANUP, "out of memory");

        query->db = db;
        query->count_stmt = stmt_clusters;
        query->no_fire_stmt = stmt_no_fire;
    }

    return query;

ERR_CLEANUP:

    free(query);
    sqlite3_finalize(stmt_clusters);
    sqlite3_finalize(stmt_no_fire);
    close_database(&db);

    return 0;
}

int
satfire_cluster_db_finalize_query_present(struct SFClusterDatabaseQueryPresent **stmt)
{
    static_assert(SQLITE_OK == 0, "SQLITE_OK must equal 0 or we'll have problems here.");

    assert(stmt && *stmt);

    int rc = SQLITE_OK;

    int rc_x = sqlite3_finalize((*stmt)->no_fire_stmt);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error finalizing no fire query statement: %s",
           sqlite3_errstr(rc_x));

    rc_x = sqlite3_finalize((*stmt)->count_stmt);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error finalizing cluster count statement: %s",
           sqlite3_errstr(rc_x));

    rc_x = close_database(&(*stmt)->db);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error closing database connection: %s",
           sqlite3_errstr(rc_x));

    free(*stmt);
    *stmt = 0;

    return rc;
}

int
satfire_cluster_db_present(struct SFClusterDatabaseQueryPresent *stmt, enum SFSatellite satellite,
                           enum SFSector sector, time_t start, time_t end)
{
    int rc = SQLITE_OK;
    int num_rows = -2; // indicates an error return value.

    rc = sqlite3_bind_text(stmt->count_stmt, 1, satfire_satellite_name(satellite), -1, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding satellite: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_text(stmt->count_stmt, 2, satfire_sector_name(sector), -1, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding sector: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_int64(stmt->count_stmt, 3, start);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_int64(stmt->count_stmt, 4, end);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s", sqlite3_errstr(rc));

    rc = sqlite3_step(stmt->count_stmt);
    Stopif(rc != SQLITE_ROW, goto ERR_CLEANUP, "Error stepping: %s (%s, %u)", sqlite3_errstr(rc),
           __FILE__, __LINE__);

    num_rows = sqlite3_column_int64(stmt->count_stmt, 0);

    rc = sqlite3_reset(stmt->count_stmt);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error resetting: %s", sqlite3_errstr(rc));

    if (num_rows == 0) {
        rc = sqlite3_bind_text(stmt->no_fire_stmt, 1, satfire_satellite_name(satellite), -1, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding satellite: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_bind_text(stmt->no_fire_stmt, 2, satfire_sector_name(sector), -1, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding sector: %s", sqlite3_errstr(rc));

        rc = sqlite3_bind_int64(stmt->no_fire_stmt, 3, start);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_bind_int64(stmt->no_fire_stmt, 4, end);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_step(stmt->no_fire_stmt);
        Stopif(rc != SQLITE_ROW, goto ERR_CLEANUP, "Error stepping: %s (%s, %u)",
               sqlite3_errstr(rc), __FILE__, __LINE__);

        num_rows = sqlite3_column_int64(stmt->no_fire_stmt, 0);

        if (num_rows > 0) {
            num_rows = 0;
        } else {
            num_rows = -1;
        }

        rc = sqlite3_reset(stmt->no_fire_stmt);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error resetting: %s", sqlite3_errstr(rc));
    }

    return num_rows;

ERR_CLEANUP:

    sqlite3_reset(stmt->count_stmt);
    sqlite3_reset(stmt->no_fire_stmt);
    return -2;
}

/*-------------------------------------------------------------------------------------------------
 *                            Query rows from the Cluster Database
 *-----------------------------------------------------------------------------------------------*/
struct SFClusterDatabaseQueryRows {
    sqlite3 *db;
    sqlite3_stmt *row_stmt;
};

struct SFClusterRow {
    time_t start;
    time_t end;
    double power;
    double scan_angle;
    enum SFSector sector;
    enum SFSatellite sat;
    struct SFPixelList *pixels;
};

struct SFClusterDatabaseQueryRows *
satfire_cluster_db_query_rows(char const *path_to_db, enum SFSatellite const sat,
                              enum SFSector const sector, time_t const start, time_t const end,
                              struct SFBoundingBox const area)
{
    assert(path_to_db);

    sqlite3 *db = 0;
    sqlite3_stmt *row_stmt = 0;
    struct SFClusterDatabaseQueryRows *query = 0;

    db = open_database_readonly(path_to_db);
    Stopif(!db, goto ERR_CLEANUP, "Unable to open database: %s", path_to_db);

    char *query_format =
        "SELECT satellite, sector, start_time, end_time, power, max_scan_angle, pixels \n"
        "FROM clusters                                                                 \n"
        "WHERE start_time >= %ld AND end_time <= %ld AND lat >= %lf AND lat <= %lf AND \n"
        "      lon >= %lf AND lon <= %lf %s %s                                         \n"
        "ORDER BY start_time ASC";

    int num_chars = 0;
    char satellite_select[32] = {0};
    if (sat != SATFIRE_SATELLITE_NONE) {
        num_chars = snprintf(satellite_select, sizeof(satellite_select), "AND satellite = '%s'",
                             satfire_satellite_name(sat));
        Stopif(num_chars >= sizeof(satellite_select), goto ERR_CLEANUP,
               "satellite select buffer too small.");
    }

    char sector_select[32] = {0};
    if (sector != SATFIRE_SECTOR_NONE) {
        num_chars = snprintf(sector_select, sizeof(sector_select), "AND sector = '%s'",
                             satfire_sector_name(sector));
        Stopif(num_chars >= sizeof(sector_select), goto ERR_CLEANUP,
               "sector select buffer too small.");
    }

    char query_txt[512] = {0};
    double min_lat = area.ll.lat;
    double min_lon = area.ll.lon;
    double max_lat = area.ur.lat;
    double max_lon = area.ur.lon;

    num_chars = snprintf(query_txt, sizeof(query_txt), query_format, start, end, min_lat, max_lat,
                         min_lon, max_lon, satellite_select, sector_select);
    Stopif(num_chars >= sizeof(query_txt), goto ERR_CLEANUP, "query_txt buffer too small: %s %d",
           __FILE__, __LINE__);

    int rc = sqlite3_prepare_v2(db, query_txt, -1, &row_stmt, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing query:\n%s\n\n%s", query_txt,
           sqlite3_errstr(rc));

    query = malloc(sizeof(struct SFClusterDatabaseQueryRows));
    Stopif(!query, exit(EXIT_FAILURE), "Out of memory.");

    query->db = db;
    query->row_stmt = row_stmt;

    return query;

ERR_CLEANUP:
    free(query);
    sqlite3_finalize(row_stmt);
    sqlite3_close(db);

    return 0;
}

int
satfire_cluster_db_query_rows_finalize(struct SFClusterDatabaseQueryRows **query)
{
    static_assert(SQLITE_OK == 0, "SQLITE_OK must equal 0 or we'll have problems here.");
    assert(query);

    int rc = SQLITE_OK;

    rc = sqlite3_finalize((*query)->row_stmt);
    sqlite3_close((*query)->db);
    free(*query);
    *query = 0;

    return rc;
}

struct SFClusterRow *
satfire_cluster_db_query_rows_next(struct SFClusterDatabaseQueryRows *query,
                                   struct SFClusterRow *old_row)
{
    assert(query);

    int rc = sqlite3_step(query->row_stmt);
    assert(rc == SQLITE_ROW || rc == SQLITE_DONE); // Fail fast in debug mode
    Stopif(rc != SQLITE_ROW && rc != SQLITE_DONE, rc = SQLITE_DONE, "Error stepping query row: %s",
           sqlite3_errstr(rc));

    struct SFClusterRow *row = old_row;
    if (rc == SQLITE_DONE) {
        satfire_cluster_db_satfire_cluster_row_finalize(row);
        row = 0;
        return row;
    }

    if (!row) {
        row = calloc(1, sizeof(struct SFClusterRow));
        Stopif(!row, exit(EXIT_FAILURE), "out of memory");
    }

    row->sat = satfire_satellite_string_contains_satellite(
        (char const *)sqlite3_column_text(query->row_stmt, 0));
    row->sector = satfire_sector_string_contains_sector(
        (char const *)sqlite3_column_text(query->row_stmt, 1));
    row->start = sqlite3_column_int64(query->row_stmt, 2);
    row->end = sqlite3_column_int64(query->row_stmt, 3);
    row->power = sqlite3_column_double(query->row_stmt, 4);
    row->scan_angle = sqlite3_column_double(query->row_stmt, 5);
    row->pixels = satfire_pixel_list_destroy(row->pixels);
    row->pixels = satfire_pixel_list_binary_deserialize(sqlite3_column_blob(query->row_stmt, 6));

    return row;
}

time_t
satfire_cluster_db_satfire_cluster_row_start(struct SFClusterRow const *row)
{
    assert(row);
    return row->start;
}

time_t
satfire_cluster_db_satfire_cluster_row_end(struct SFClusterRow const *row)
{
    assert(row);
    return row->end;
}

double
satfire_cluster_db_satfire_cluster_row_power(struct SFClusterRow const *row)
{
    assert(row);
    return row->power;
}

double
satfire_cluster_db_satfire_cluster_row_scan_angle(struct SFClusterRow const *row)
{
    assert(row);
    return row->scan_angle;
}

enum SFSatellite
satfire_cluster_db_satfire_cluster_row_satellite(struct SFClusterRow const *row)
{
    assert(row);
    return row->sat;
}

enum SFSector
satfire_cluster_db_satfire_cluster_row_sector(struct SFClusterRow const *row)
{
    assert(row);
    return row->sector;
}

const struct SFPixelList *
satfire_cluster_db_satfire_cluster_row_pixels(struct SFClusterRow const *row)
{
    assert(row);
    return row->pixels;
}

void
satfire_cluster_db_satfire_cluster_row_finalize(struct SFClusterRow *row)
{
    if (row) {
        row->pixels = satfire_pixel_list_destroy(row->pixels);
        free(row);
    }
}
