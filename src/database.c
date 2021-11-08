#include "database.h"

#include <assert.h>
#include <stdbool.h>
#include <stdlib.h>

#include <sqlite3.h>

#include "util.h"

/*-------------------------------------------------------------------------------------------------
 *                                 ClusterDatabase Open/Close
 *-----------------------------------------------------------------------------------------------*/
struct ClusterDatabase {
    sqlite3 *ptr;
};

struct ClusterDatabase *
cluster_db_connect(char const *path)
{
    sqlite3 *handle = 0;
    int rc = sqlite3_open_v2(path, &handle,
                             SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_NOMUTEX, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error connecting to %s", path);

    // A 5-second busy time out is WAY too much. If we hit this something has gone terribly wrong.
    sqlite3_busy_timeout(handle, 5000);

    char *query = "CREATE TABLE IF NOT EXISTS clusters (             \n"
                  "  satellite  TEXT    NOT NULL,                    \n"
                  "  sector     TEXT    NOT NULL,                    \n"
                  "  start_time INTEGER NOT NULL,                    \n"
                  "  end_time   INTEGER NOT NULL,                    \n"
                  "  lat        REAL    NOT NULL,                    \n"
                  "  lon        REAL    NOT NULL,                    \n"
                  "  power      REAL    NOT NULL,                    \n"
                  "  pixels     BLOB    NOT NULL);                   \n"
                  "                                                  \n"
                  "CREATE UNIQUE INDEX IF NOT EXISTS no_cluster_dups \n"
                  "  ON clusters (satellite, sector, start_time,     \n"
                  "               end_time, lat, lon);               \n"
                  "                                                  \n"
                  "CREATE INDEX IF NOT EXISTS file_processed         \n"
                  "  ON clusters (satellite, sector, start_time,     \n"
                  "               end_time);                         \n"
                  "                                                  \n"
                  "CREATE TABLE IF NOT EXISTS no_fire (              \n"
                  "  satellite  TEXT    NOT NULL,                    \n"
                  "  sector     TEXT    NOT NULL,                    \n"
                  "  start_time INTEGER NOT NULL,                    \n"
                  "  end_time   INTEGER NOT NULL);                   \n";

    char *err_message = 0;

    rc = sqlite3_exec(handle, query, 0, 0, &err_message);
    if (rc != SQLITE_OK) {
        fprintf(stderr, "Error initializing database: %s\n\n", err_message);
        sqlite3_free(err_message);
        goto ERR_CLEANUP;
    }

    struct ClusterDatabase *cdbh = malloc(sizeof(struct ClusterDatabase));
    Stopif(!cdbh, goto ERR_CLEANUP, "out of memory");
    cdbh->ptr = handle;

    return cdbh;

ERR_CLEANUP:
    sqlite3_close(handle);
    return 0;
}

int
cluster_db_close(struct ClusterDatabase **db)
{
    assert(db);

    if (*db) {
        int rc = sqlite3_close((*db)->ptr);
        free(*db);
        *db = 0;
        return rc;
    }

    return 0;
}

/*-------------------------------------------------------------------------------------------------
 *                                ClusterDatabase Adding Rows
 *-----------------------------------------------------------------------------------------------*/
struct ClusterDatabaseAdd {
    sqlite3_stmt *add_ptr;
    sqlite3_stmt *no_fire_ptr;
};

struct ClusterDatabaseAdd *
cluster_db_prepare_to_add(struct ClusterDatabase *db)
{
    assert(db);

    struct ClusterDatabaseAdd *add = 0;
    sqlite3_stmt *add_stmt = 0;
    sqlite3_stmt *no_fire_stmt = 0;

    char *add_query = "INSERT OR REPLACE INTO clusters (                                  \n"
                      "  satellite, sector, start_time, end_time, lat, lon, power, pixels)\n"
                      "VALUES (?, ?, ?, ?, ?, ?, ?, ?)                                    \n";

    int rc = sqlite3_prepare_v2(db->ptr, add_query, -1, &add_stmt, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing statement: %s", sqlite3_errstr(rc));

    char *no_fire_query = "INSERT OR REPLACE INTO no_fire              \n"
                          "  (satellite, sector, start_time, end_time) \n"
                          "VALUES (?, ?, ?, ?)                         \n";

    rc = sqlite3_prepare_v2(db->ptr, no_fire_query, -1, &no_fire_stmt, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing statement: %s", sqlite3_errstr(rc));

    add = malloc(sizeof(struct ClusterDatabaseAdd));
    Stopif(!add, goto ERR_CLEANUP, "out of memory");

    add->add_ptr = add_stmt;
    add->no_fire_ptr = no_fire_stmt;

    return add;

ERR_CLEANUP:
    free(add);
    sqlite3_finalize(add_stmt);
    sqlite3_finalize(no_fire_stmt);

    return 0;
}

int
cluster_db_finalize_add(struct ClusterDatabase *db, struct ClusterDatabaseAdd **stmt)
{
    static_assert(SQLITE_OK == 0, "SQLITE_OK must equal 0 or we'll have problems here.");

    assert(db && db->ptr && stmt && (*stmt) && (*stmt)->add_ptr && (*stmt)->no_fire_ptr);

    int rc = SQLITE_OK;
    int rc_x = sqlite3_finalize((*stmt)->add_ptr);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error finalizing add statement: %s",
           sqlite3_errstr(rc_x));
    rc_x = sqlite3_finalize((*stmt)->no_fire_ptr);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error finalizing no fire statement: %s",
           sqlite3_errstr(rc_x));

    free(*stmt);
    *stmt = 0;

    return rc;
}

static int
cluster_db_add_cluster(struct ClusterDatabase *db, struct ClusterDatabaseAdd *stmt,
                       struct ClusterList *clist)
{
    assert(stmt && stmt->add_ptr && clist);

    int rc = SQLITE_OK;
    char *err_message = 0;

    char *begin_trans = "BEGIN TRANSACTION";
    rc = sqlite3_exec(db->ptr, begin_trans, 0, 0, &err_message);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error starting transaction: %s", err_message);

    char const *satellite = cluster_list_satellite(clist);
    char const *sector = cluster_list_sector(clist);
    time_t scan_start = cluster_list_scan_start(clist);
    time_t scan_end = cluster_list_scan_end(clist);

    GArray *clusters = cluster_list_clusters(clist);

    unsigned char buffer[1024] = {0};

    for (unsigned int i = 0; i < clusters->len; ++i) {

        struct Cluster *cluster = g_array_index(clusters, struct Cluster *, i);

        rc = sqlite3_bind_text(stmt->add_ptr, 1, satellite, -1, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding satellite: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_bind_text(stmt->add_ptr, 2, sector, -1, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding sector: %s", sqlite3_errstr(rc));

        rc = sqlite3_bind_int64(stmt->add_ptr, 3, scan_start);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_bind_int64(stmt->add_ptr, 4, scan_end);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding start time: %s",
               sqlite3_errstr(rc));

        struct Coord centroid = cluster_centroid(cluster);

        rc = sqlite3_bind_double(stmt->add_ptr, 5, centroid.lat);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding lat: %s", sqlite3_errstr(rc));

        rc = sqlite3_bind_double(stmt->add_ptr, 6, centroid.lon);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding lon: %s", sqlite3_errstr(rc));

        rc = sqlite3_bind_double(stmt->add_ptr, 7, cluster_total_power(cluster));
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding power: %s", sqlite3_errstr(rc));

        unsigned char *buf_ptr = buffer;
        void (*transient_free)(void *) = SQLITE_TRANSIENT;
        size_t buff_size = pixel_list_binary_serialize_buffer_size(cluster_pixels(cluster));
        if (buff_size > sizeof(buffer)) {
            transient_free = free; // free function from stdlib.h
            buf_ptr = calloc(buff_size, sizeof(unsigned char));
            Stopif(!buf_ptr, exit(EXIT_FAILURE), "calloc failure: out of memory");
        }

        size_t num_bytes_serialized =
            pixel_list_binary_serialize(cluster_pixels(cluster), buff_size, buf_ptr);
        Stopif(num_bytes_serialized != buff_size, exit(EXIT_FAILURE),
               "Buffer size error serializing PixelList");
        rc = sqlite3_bind_blob(stmt->add_ptr, 8, buf_ptr, buff_size, transient_free);

        rc = sqlite3_step(stmt->add_ptr);
        Stopif(rc != SQLITE_OK && rc != SQLITE_DONE, goto ERR_CLEANUP,
               "Error stepping: %s (%s, %u)", sqlite3_errstr(rc), __FILE__, __LINE__);

        rc = sqlite3_reset(stmt->add_ptr);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error resetting: %s", sqlite3_errstr(rc));
    }

    char *commit_trans = "COMMIT TRANSACTION";
    rc = sqlite3_exec(db->ptr, commit_trans, 0, 0, &err_message);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error committing transaction: %s", err_message);

    return 0;

ERR_CLEANUP:

    sqlite3_reset(stmt->add_ptr);
    sqlite3_free(err_message);
    return 1;
}

static int
cluster_db_add_no_fire(struct ClusterDatabase *db, struct ClusterDatabaseAdd *stmt,
                       struct ClusterList *clist)
{
    assert(stmt && stmt->no_fire_ptr && clist);

    int rc = SQLITE_OK;
    char *err_message = 0;

    char const *satellite = cluster_list_satellite(clist);
    char const *sector = cluster_list_sector(clist);
    time_t scan_start = cluster_list_scan_start(clist);
    time_t scan_end = cluster_list_scan_end(clist);

    rc = sqlite3_bind_text(stmt->no_fire_ptr, 1, satellite, -1, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding satellite: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_text(stmt->no_fire_ptr, 2, sector, -1, 0);
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
cluster_db_add(struct ClusterDatabase *db, struct ClusterDatabaseAdd *stmt,
               struct ClusterList *clist)
{
    GArray *clusters = cluster_list_clusters(clist);
    if (clusters->len > 0) {
        return cluster_db_add_cluster(db, stmt, clist);
    } else {
        return cluster_db_add_no_fire(db, stmt, clist);
    }
}

/*-------------------------------------------------------------------------------------------------
 *                                   ClusterDatabase Query
 *-----------------------------------------------------------------------------------------------*/
time_t
cluster_db_newest_scan_start(struct ClusterDatabase *db, char const *satellite, char const *sector)
{
    time_t newest_scan_time = 0;
    char *query = 0;
    asprintf(&query,
             "SELECT MAX(start_time) FROM clusters WHERE satellite = '%s' AND sector = '%s'",
             satellite, sector);

    sqlite3_stmt *stmt = 0;
    int rc = sqlite3_prepare_v2(db->ptr, query, -1, &stmt, 0);
    Stopif(rc != SQLITE_OK, goto CLEAN_UP, "Error preparing newest scan statement: %s",
           sqlite3_errstr(rc));

    rc = sqlite3_step(stmt);
    Stopif(rc != SQLITE_ROW, goto CLEAN_UP, "Error stepping: %s", sqlite3_errstr(rc));

    // Check for NULL
    if (sqlite3_column_type(stmt, 0) != SQLITE_INTEGER) {
        goto CLEAN_UP;
    }

    newest_scan_time = sqlite3_column_int64(stmt, 0);

CLEAN_UP:
    free(query);
    rc = sqlite3_finalize(stmt);
    Stopif(rc != SQLITE_OK, return newest_scan_time, "Error finalizing: %s", sqlite3_errstr(rc));

    return newest_scan_time;
}

struct ClusterDatabaseQueryPresent {
    sqlite3_stmt *count_stmt;
    sqlite3_stmt *no_fire_stmt;
};

struct ClusterDatabaseQueryPresent *
cluster_database_prepare_to_query_present(ClusterDatabaseH db)
{
    int rc = SQLITE_OK;
    struct ClusterDatabaseQueryPresent *query = 0;
    sqlite3_stmt *stmt_clusters = 0;
    sqlite3_stmt *stmt_no_fire = 0;

    char *query_clusters =
        "SELECT COUNT(*) FROM clusters                                         \n"
        "WHERE satellite = ? AND sector = ? AND start_time = ? AND end_time = ?\n";

    rc = sqlite3_prepare_v2(db->ptr, query_clusters, -1, &stmt_clusters, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing count rows statement: %s",
           sqlite3_errstr(rc));

    char *query_no_fire =
        "SELECT COUNT(*) FROM no_fire                                          \n"
        "WHERE satellite = ? AND sector = ? AND start_time = ? AND end_time = ?\n";

    rc = sqlite3_prepare_v2(db->ptr, query_no_fire, -1, &stmt_no_fire, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error preparing count rows statement: %s",
           sqlite3_errstr(rc));

    query = malloc(sizeof(struct ClusterDatabaseQueryPresent));
    Stopif(!query, goto ERR_CLEANUP, "out of memory");

    query->count_stmt = stmt_clusters;
    query->no_fire_stmt = stmt_no_fire;

    return query;

ERR_CLEANUP:

    free(query);
    sqlite3_finalize(stmt_clusters);
    sqlite3_finalize(stmt_no_fire);

    return 0;
}

int
cluster_db_finalize_query_present(ClusterDatabaseH db, ClusterDatabaseQueryPresentH *stmt)
{
    static_assert(SQLITE_OK == 0, "SQLITE_OK must equal 0 or we'll have problems here.");

    assert(db && db->ptr && stmt && (*stmt) && (*stmt)->count_stmt && (*stmt)->no_fire_stmt);

    int rc = SQLITE_OK;
    int rc_x = sqlite3_finalize((*stmt)->no_fire_stmt);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error finalizing no fire query statement: %s",
           sqlite3_errstr(rc_x));
    rc_x = sqlite3_finalize((*stmt)->count_stmt);
    Stopif(rc_x != SQLITE_OK, rc = rc_x, "Error finalizing cluster count statement: %s",
           sqlite3_errstr(rc_x));

    free(*stmt);
    *stmt = 0;

    return rc;
}

int
cluster_db_present(struct ClusterDatabaseQueryPresent *stmt, char const *satellite,
                   char const *sector, time_t start, time_t end)
{
    int rc = SQLITE_OK;
    int num_rows = -2; // indicates an error return value.

    rc = sqlite3_bind_text(stmt->count_stmt, 1, satellite, -1, 0);
    Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding satellite: %s", sqlite3_errstr(rc));

    rc = sqlite3_bind_text(stmt->count_stmt, 2, sector, -1, 0);
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
        rc = sqlite3_bind_text(stmt->no_fire_stmt, 1, satellite, -1, 0);
        Stopif(rc != SQLITE_OK, goto ERR_CLEANUP, "Error binding satellite: %s",
               sqlite3_errstr(rc));

        rc = sqlite3_bind_text(stmt->no_fire_stmt, 2, sector, -1, 0);
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
