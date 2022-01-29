
fn main() {
    println!("Hello world.");
}

/*
/** \file findfire.c
 * \brief Group individual satellite pixels showing wildfire into connected clusters.
 *
 * This program walks a directory tree and analyzes all the NOAA Big Data files with GOES satellite
 * Fire Detection Characteristics (FDC) data. Individual pixels with fire power greater than 0.0 MW
 * are grouped into clusters of adjacent pixels. The power is summed to get a total power for the
 * cluster, and then the total power and a geographic description of all the pixels in the cluster
 * are serialized and stored in a database. The source satellite, scanning sector (Full Disk, CONUS,
 * MesoSector), scan start, and scan end times are also stored in the database with each cluster.
 *
 * The goal of having all this data together is for other programs to read the data from the
 * database and perform more analysis.
 *
 * This program queries an existing database to find if a file has been processed already before
 * processing it.
 *
 * At the end of processing, some summary statistics are printed to the screen and a file called
 * findfire.kml is output in the same location as the database file findfire.sqlite that has the
 * largest Cluster processed this time.
 */
// Standard C
#include <assert.h>
#include <limits.h>
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

// System installed libraries
#include <glib.h>

// My headers
#include "satfire.h"

// Source Libraries
#include "courier.h"
#include "kamel.h"
#include "sf_util.h"

#if defined(__APPLE__) && defined(__MACH__)
#    define pthread_setname(a)
#elif defined(__linux__)
#    define pthread_setname(a) pthread_setname_np(pthread_self(), (a))
#endif

/*-------------------------------------------------------------------------------------------------
 *                          Program Initialization, Finalization, and Options
 *-----------------------------------------------------------------------------------------------*/
static struct FindFireOptions {
    char *database_file;
    char *kml_file;
    char *data_dir;
    bool only_new;
    bool verbose;

} options = {0};

// clang-format off
static GOptionEntry option_entries[] = 
{
    {
        "new", 
        'n', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_NONE, 
        &options.only_new, 
        "Only try to find data newer than what's already in the database for each "
            "satellite and sector.", 
        0
    },
    {
        "verbose", 
        'v', 
        G_OPTION_FLAG_NONE, 
        G_OPTION_ARG_NONE, 
        &options.verbose, 
        "Show verbose output.", 
        0
    },

    {NULL}
};
// clang-format on

static void
program_initialization(int argc[static 1], char ***argv)
{
    // Force to use UTC timezone.
    setenv("TZ", "UTC", 1);
    tzset();

    satfire_initialize();

    // Initialize with with environment variables and default values.
    if (getenv("CLUSTER_DB")) {
        asprintf(&options.database_file, "%s", getenv("CLUSTER_DB"));
        asprintf(&options.kml_file, "%s.kml", options.database_file);
    }

    if (getenv("SAT_ARCHIVE")) {
        asprintf(&options.data_dir, "%s", getenv("SAT_ARCHIVE"));
    }

    options.only_new = false;

    // Parse command line options.
    GError *error = 0;
    GOptionContext *context = g_option_context_new("- Find clusters and add them to a database.");
    g_option_context_add_main_entries(context, option_entries, 0);
    g_option_context_parse(context, argc, argv, &error);
    Stopif(error, exit(EXIT_FAILURE), "Error parsing options: %s", error->message);
    g_option_context_free(context);

    Stopif(!options.database_file, exit(EXIT_FAILURE), "Invalid, database_file is NULL");
    Stopif(!options.data_dir, exit(EXIT_FAILURE), "Invalid, data_dir is NULL");

    // Print out options as configured.
    if (options.verbose) {
        fprintf(stdout, "  Database: %s\n", options.database_file);
        if (options.kml_file) {
            fprintf(stdout, "Output KML: %s\n", options.kml_file);
        }
        fprintf(stdout, "   Archive: %s\n", options.data_dir);
        fprintf(stdout, "  Only New: %s\n", options.only_new ? "yes" : "no");
    }

    satfire_db_initialize(options.database_file);
}

static void
program_finalization()
{
    free(options.database_file);
    free(options.kml_file);
    free(options.data_dir);

    satfire_finalize();
}

/*-------------------------------------------------------------------------------------------------
 *                       Filters for skipping files / directories
 *-----------------------------------------------------------------------------------------------*/
static bool
standard_dir_filter(char const *path, void *user_data)
{
    assert(path);
    assert(user_data);

    /* This filter assumes the data is stored in a directory tree like:
     *   SATELLITE/SECTOR/YEAR/DAY_OF_YEAR/HOUR/files
     *
     *   e.g.
     *   G16/ABI-L2-FDCF/2020/238/15/...files...
     */
    struct tm *most_recent_data = user_data;

    enum SFSatellite sat = satfire_satellite_string_contains_satellite(path);
    enum SFSector sector = satfire_sector_string_contains_sector(path);
    if (sat == SATFIRE_SATELLITE_NONE || sector == SATFIRE_SECTOR_NONE) {
        // Maybe we need to recurse deeper to be sure...
        return true;
    }

    struct tm most_recent = most_recent_data[sat * SATFIRE_SECTOR_NUM + sector];
    int mr_year = most_recent.tm_year + 1900;
    int mr_doy = most_recent.tm_yday + 1;
    int mr_hour = most_recent.tm_hour;

    // Find the year and the day of the year in the string.
    char const *c = path;
    int year = -1;
    int doy = -1;
    int hour = -1;
    while (c && *c) {
        int maybe = atoi(c);
        if (maybe > 2000) {
            year = maybe;
        } else if (maybe > 0 || *c == '0') {
            if (doy == -1) {
                doy = maybe;
            } else {
                hour = maybe;
                break;
            }
        }
        c = strchr(c, '/');
        if (c && *c) {
            c += 1;
        }
    }

    if (year == -1) {
        // Not deep enough to parse year, keep going.
        return true;
    } else if (year < mr_year) {
        // In a past year, recurse no more deeply!
        return false;
    } else if (doy == -1) {
        // Not deep enough to parse day of year, keep going.
        return true;
    } else if (doy < mr_doy && year <= mr_year) {
        // Same year, but sooner in the year for most recent, recurse no more deeply!
        return false;
    } else if (hour == -1) {
        // Not deep enough to parse hour of day, keep going.
        return true;
    } else if (hour < mr_hour && doy <= mr_doy && year <= mr_year) {
        // Same year, same day of year, but too early in the day, recurse no more deeply!
        return false;
    }

    // We must be near the present or the future, so keep going!
    return true;
}

static bool
skip_path(char const *path, SFClusterDatabaseQueryPresentH query)
{
    char const *ext = file_ext(path);

    if ((strcmp("zip", ext) != 0 && strcmp("nc", ext) != 0) || strstr(path, ".txt") != 0) {
        // Only process files with the '.nc' or the '.zip' extension, and that don't have '.txt'
        // in the file name.
        return true;
    }

    enum SFSatellite satellite = satfire_satellite_string_contains_satellite(path);
    enum SFSector sector = satfire_sector_string_contains_sector(path);

    if (satellite == SATFIRE_SATELLITE_NONE || sector == SATFIRE_SECTOR_NONE) {
        return true;
    }

    // Skip meso-sectors for now, I don't have many of those.
    // if (sector == SATFIRE_SECTOR_MESO1 || sector == SATFIRE_SECTOR_MESO2) {
    //    return true;
    //}

    // TODO: Need to add error checking to here. On error, parse_time_string should return the
    // time 0 since that is way out of bounds for the GOES R/S/T/.... era.
    time_t scan_start = parse_time_string(satfire_cluster_find_start_time(path));
    time_t scan_end = parse_time_string(satfire_cluster_find_end_time(path));

    int num_rows = satfire_cluster_db_present(query, satellite, sector, scan_start, scan_end);
    Stopif(num_rows < -1, return false, "Error querying num_rows, proceeding anyway.");

    if (num_rows >= 0) {
        return true;
    }

    return false;
}

/*-------------------------------------------------------------------------------------------------
 *                             Save a Cluster in a KML File
 *-----------------------------------------------------------------------------------------------*/
static void
output_cluster_kml(FILE *out, char const *name, struct SFCluster *cluster, time_t start, time_t end,
                   enum SFSatellite sat, enum SFSector sector)
{

    assert(name);
    assert(out);
    assert(cluster);

    kamel_start_folder(out, name, 0, true);
    kamel_timespan(out, start, end);

    char *description = 0;
    asprintf(&description,
             "Satellite: %s<br/>"
             "Sector: %s<br/>"
             "Power: %.0lf MW<br/>"
             "Area: %.0lf m^2<br/>"
             "Max Temperature: %.0lf&deg;K",
             satfire_satellite_name(sat), satfire_sector_name(sector),
             satfire_cluster_total_power(cluster), satfire_cluster_total_area(cluster),
             satfire_cluster_max_temperature(cluster));

    kamel_start_placemark(out, name, description, "#fire");
    struct SFCoord centroid = satfire_pixel_list_centroid(satfire_cluster_pixels(cluster));
    kamel_point(out, centroid.lat, centroid.lon, 0.0);
    kamel_end_placemark(out);
    free(description);

    satfire_pixel_list_kml_write(out, satfire_cluster_pixels(cluster));

    kamel_end_folder(out);

    return;
}

static void
save_satfire_cluster_kml(struct SFCluster *biggest, time_t bstart, time_t bend,
                         enum SFSatellite bsat, enum SFSector bsector, struct SFCluster *hottest,
                         time_t hstart, time_t hend, enum SFSatellite hsat, enum SFSector hsector)
{
    // Return early if no output file is configured.
    if (!options.kml_file) {
        return;
    }

    FILE *out = fopen(options.kml_file, "wb");
    Stopif(!out, return, "Unable to open file for writing: %s", options.kml_file);

    kamel_start_document(out);

    kamel_start_style(out, "fire");
    kamel_poly_style(out, "880000FF", true, false);
    kamel_icon_style(out, "http://maps.google.com/mapfiles/kml/shapes/firedept.png", 1.3);
    kamel_end_style(out);

    output_cluster_kml(out, "Biggest Fire", biggest, bstart, bend, bsat, bsector);
    output_cluster_kml(out, "Hottest Fire", hottest, hstart, hend, hsat, hsector);

    kamel_end_document(out);

    fclose(out);

    return;
}

/*-------------------------------------------------------------------------------------------------
 *                               Cluster and Image Statistics
 *-----------------------------------------------------------------------------------------------*/
/* Use this as the maximum value of the scan angle allowed for a cluster to be considered in the
 * summary statistics. This is a QC tool, there are a lot of outliers on the limb of the Earth as
 * viewed by the GOES satellites, and the angles / geometry seem to have something to do with it.
 *
 * The value of 8.3 degrees is based on visual inspection of a graph of cluster power vs max scan
 * angle of the cluster member centroids.
 */
#define MAX_SCAN_ANGLE 8.3

struct ClusterStats {
    struct SFCluster *biggest_fire;
    enum SFSatellite biggest_sat;
    enum SFSector biggest_sector;
    time_t biggest_start;
    time_t biggest_end;

    struct SFCluster *hottest_fire;
    enum SFSatellite hottest_sat;
    enum SFSector hottest_sector;
    time_t hottest_start;
    time_t hottest_end;

    unsigned num_clusters;
    unsigned num_power_lt_1mw;
    unsigned num_power_lt_10mw;
    unsigned num_power_lt_100mw;
    unsigned num_power_lt_1gw;
    unsigned num_power_lt_10gw;
    unsigned num_power_lt_100gw;
};

static struct ClusterStats
satfire_cluster_stats_new(void)
{
    return (struct ClusterStats){
        .biggest_fire = satfire_cluster_new(),
        .biggest_sat = SATFIRE_SATELLITE_NONE,
        .biggest_sector = SATFIRE_SECTOR_NONE,
        .biggest_start = 0,
        .biggest_end = 0,
        .hottest_fire = satfire_cluster_new(),
        .hottest_sat = SATFIRE_SATELLITE_NONE,
        .hottest_sector = SATFIRE_SECTOR_NONE,
        .hottest_start = 0,
        .hottest_end = 0,
        .num_clusters = 0,
        .num_power_lt_1mw = 0,
        .num_power_lt_10mw = 0,
        .num_power_lt_100mw = 0,
        .num_power_lt_1gw = 0,
        .num_power_lt_10gw = 0,
        .num_power_lt_100gw = 0,
    };
}

static void
satfire_cluster_stats_destroy(struct ClusterStats *tgt)
{
    satfire_cluster_destroy(&tgt->biggest_fire);
    satfire_cluster_destroy(&tgt->hottest_fire);
    memset(tgt, 0, sizeof(struct ClusterStats));
}

static struct ClusterStats
satfire_cluster_stats_update(struct ClusterStats stats, enum SFSatellite sat, enum SFSector sector,
                             time_t start, time_t end, struct SFCluster *cluster)
{
    double satfire_cluster_power = satfire_cluster_total_power(cluster);
    double satfire_cluster_temperature = satfire_cluster_max_temperature(cluster);

    if (satfire_cluster_max_scan_angle(cluster) < MAX_SCAN_ANGLE) {
        if (satfire_cluster_power > satfire_cluster_total_power(stats.biggest_fire)) {
            satfire_cluster_destroy(&stats.biggest_fire);
            stats.biggest_fire = satfire_cluster_copy(cluster);
            stats.biggest_sat = sat;
            stats.biggest_sector = sector;
            stats.biggest_start = start;
            stats.biggest_end = end;
        }

        if (satfire_cluster_temperature > satfire_cluster_max_temperature(stats.hottest_fire)) {
            satfire_cluster_destroy(&stats.hottest_fire);
            stats.hottest_fire = satfire_cluster_copy(cluster);
            stats.hottest_sat = sat;
            stats.hottest_sector = sector;
            stats.hottest_start = start;
            stats.hottest_end = end;
        }

        if (satfire_cluster_power < 1.0) {
            stats.num_power_lt_1mw += 1;
        }

        if (satfire_cluster_power < 10.0) {
            stats.num_power_lt_10mw += 1;
        }

        if (satfire_cluster_power < 100.0) {
            stats.num_power_lt_100mw += 1;
        }

        if (satfire_cluster_power < 1000.0) {
            stats.num_power_lt_1gw += 1;
        }

        if (satfire_cluster_power < 10000.0) {
            stats.num_power_lt_10gw += 1;
        }

        if (satfire_cluster_power < 100000.0) {
            stats.num_power_lt_100gw += 1;
        }

        stats.num_clusters += 1;
    }

    return stats;
}

static void
satfire_cluster_stats_print(struct ClusterStats stats)
{
    if (stats.num_clusters > 0) {

        char power_start_str[128] = {0};
        ctime_r(&stats.biggest_start, power_start_str);
        char power_end_str[128] = {0};
        ctime_r(&stats.biggest_end, power_end_str);

        struct SFCoord biggest_centroid = satfire_cluster_centroid(stats.biggest_fire);

        char hot_start_str[128] = {0};
        ctime_r(&stats.hottest_start, hot_start_str);
        char hot_end_str[128] = {0};
        ctime_r(&stats.hottest_end, hot_end_str);

        struct SFCoord hot_centroid = satfire_cluster_centroid(stats.hottest_fire);

        printf("\nIndividual Cluster Stats\n\n"
               " Most Powerfull:\n"
               "      satellite: %s\n"
               "         sector: %s\n"
               "          start: %s"
               "            end: %s"
               "            Lat: %10.6lf\n"
               "            Lon: %11.6lf\n"
               " Max Scan Angle: %3.0lf\n"
               "          Count: %2d\n"
               "          Power: %5.0lf MW\n"
               "           Area: %5.0lf square kilometers\n"
               "Max Temperature: %5.0lf Kelvin\n\n"
               "        Hottest:\n"
               "      satellite: %s\n"
               "         sector: %s\n"
               "          start: %s"
               "            end: %s"
               "            Lat: %10.6lf\n"
               "            Lon: %11.6lf\n"
               " Max Scan Angle: %3.0lf\n"
               "          Count: %2d\n"
               "          Power: %5.0lf MW\n"
               "           Area: %5.0lf square kilometers\n"
               "Max Temperature: %5.0lf Kelvin\n\n"
               "        Counts:\n"
               "         Total: %10u\n"
               "Power <   1 MW: %10u\n"
               "Power <  10 MW: %10u\n"
               "Power < 100 MW: %10u\n"
               "Power <   1 GW: %10u\n"
               "Power <  10 GW: %10u\n"
               "Power < 100 GW: %10u\n\n"
               "  Pct <   1 MW: %10u%%\n"
               "  Pct <  10 MW: %10u%%\n"
               "  Pct < 100 MW: %10u%%\n"
               "  Pct <   1 GW: %10u%%\n"
               "  Pct <  10 GW: %10u%%\n"
               "  Pct < 100 GW: %10u%%\n",
               satfire_satellite_name(stats.biggest_sat), satfire_sector_name(stats.biggest_sector),
               power_start_str, power_end_str, biggest_centroid.lat, biggest_centroid.lon,
               satfire_cluster_max_scan_angle(stats.biggest_fire),
               satfire_cluster_pixel_count(stats.biggest_fire),
               satfire_cluster_total_power(stats.biggest_fire),
               satfire_cluster_total_area(stats.biggest_fire) / (1000.0 * 1000.0),
               satfire_cluster_max_temperature(stats.biggest_fire),

               satfire_satellite_name(stats.hottest_sat), satfire_sector_name(stats.hottest_sector),
               hot_start_str, hot_end_str, hot_centroid.lat, hot_centroid.lon,
               satfire_cluster_max_scan_angle(stats.hottest_fire),
               satfire_cluster_pixel_count(stats.hottest_fire),
               satfire_cluster_total_power(stats.hottest_fire),
               satfire_cluster_total_area(stats.hottest_fire) / (1000.0 * 1000.0),
               satfire_cluster_max_temperature(stats.hottest_fire),

               stats.num_clusters, stats.num_power_lt_1mw, stats.num_power_lt_10mw,
               stats.num_power_lt_100mw, stats.num_power_lt_1gw, stats.num_power_lt_10gw,
               stats.num_power_lt_100gw, stats.num_power_lt_1mw * 100 / stats.num_clusters,
               stats.num_power_lt_10mw * 100 / stats.num_clusters,
               stats.num_power_lt_100mw * 100 / stats.num_clusters,
               stats.num_power_lt_1gw * 100 / stats.num_clusters,
               stats.num_power_lt_10gw * 100 / stats.num_clusters,
               stats.num_power_lt_100gw * 100 / stats.num_clusters);
    } else {
        printf("\nNo new clusters added to the database.");
    }
}

struct SFClusterListStats {
    enum SFSatellite min_num_clusters_sat;
    enum SFSector min_num_clusters_sector;
    unsigned int min_num_clusters;
    time_t min_num_clusters_start;
    time_t min_num_clusters_end;

    enum SFSatellite max_num_clusters_sat;
    enum SFSector max_num_clusters_sector;
    unsigned int max_num_clusters;
    time_t max_num_clusters_start;
    time_t max_num_clusters_end;

    enum SFSatellite max_total_power_sat;
    enum SFSector max_total_power_sector;
    double max_total_power;
    time_t max_total_power_start;
    time_t max_total_power_end;

    enum SFSatellite min_total_power_sat;
    enum SFSector min_total_power_sector;
    double min_total_power;
    time_t min_total_power_start;
    time_t min_total_power_end;
};

static struct SFClusterListStats
satfire_cluster_list_stats_new(void)
{
    return (struct SFClusterListStats){
        .min_num_clusters_sat = SATFIRE_SATELLITE_NONE,
        .min_num_clusters_sector = SATFIRE_SECTOR_NONE,
        .min_num_clusters = UINT_MAX,
        .min_num_clusters_start = 0,
        .min_num_clusters_end = 0,

        .max_num_clusters_sat = SATFIRE_SATELLITE_NONE,
        .max_num_clusters_sector = SATFIRE_SECTOR_NONE,
        .max_num_clusters = 0,
        .max_num_clusters_start = 0,
        .max_num_clusters_end = 0,

        .max_total_power_sat = SATFIRE_SATELLITE_NONE,
        .max_total_power_sector = SATFIRE_SECTOR_NONE,
        .max_total_power = 0.0,
        .max_total_power_start = 0,
        .max_total_power_end = 0,

        .min_total_power_sat = SATFIRE_SATELLITE_NONE,
        .min_total_power_sector = SATFIRE_SECTOR_NONE,
        .min_total_power = HUGE_VAL,
        .min_total_power_start = 0,
        .min_total_power_end = 0,
    };
}

static void
satfire_cluster_list_stats_destroy(struct SFClusterListStats *clstats)
{
    // Nothing to do at this time because nothing is heap allocated.
}

static struct SFClusterListStats
satfire_cluster_list_stats_update(struct SFClusterListStats clstats, struct SFClusterList *clusters)
{
    unsigned int num_clust = satfire_cluster_list_length(clusters);

    if (num_clust > clstats.max_num_clusters) {
        clstats.max_num_clusters = num_clust;
        clstats.max_num_clusters_sat = satfire_cluster_list_satellite(clusters);
        clstats.max_num_clusters_sector = satfire_cluster_list_sector(clusters);
        clstats.max_num_clusters_start = satfire_cluster_list_scan_start(clusters);
        clstats.max_num_clusters_end = satfire_cluster_list_scan_end(clusters);
    }

    if (num_clust < clstats.min_num_clusters) {
        clstats.min_num_clusters = num_clust;
        clstats.min_num_clusters_sat = satfire_cluster_list_satellite(clusters);
        clstats.min_num_clusters_sector = satfire_cluster_list_sector(clusters);
        clstats.min_num_clusters_start = satfire_cluster_list_scan_start(clusters);
        clstats.min_num_clusters_end = satfire_cluster_list_scan_end(clusters);
    }

    double total_power = satfire_cluster_list_total_power(clusters);
    if (total_power > clstats.max_total_power) {
        clstats.max_total_power = total_power;
        clstats.max_total_power_sat = satfire_cluster_list_satellite(clusters);
        clstats.max_total_power_sector = satfire_cluster_list_sector(clusters);
        clstats.max_total_power_start = satfire_cluster_list_scan_start(clusters);
        clstats.max_total_power_end = satfire_cluster_list_scan_end(clusters);
    }

    if (total_power < clstats.min_total_power) {
        clstats.min_total_power = total_power;
        clstats.min_total_power_sat = satfire_cluster_list_satellite(clusters);
        clstats.min_total_power_sector = satfire_cluster_list_sector(clusters);
        clstats.min_total_power_start = satfire_cluster_list_scan_start(clusters);
        clstats.min_total_power_end = satfire_cluster_list_scan_end(clusters);
    }

    return clstats;
}

static void
satfire_cluster_list_stats_print(struct SFClusterListStats clstats)
{
    char start_str[128] = {0};
    ctime_r(&clstats.max_total_power_start, start_str);
    char end_str[128] = {0};
    ctime_r(&clstats.max_total_power_end, end_str);

    printf("\n\n"
           "Max Image Power Stats:\n"
           "            satellite: %s\n"
           "               sector: %s\n"
           "                start: %s"
           "                  end: %s"
           "      Max Total Power: %.0lf GW\n\n",
           satfire_satellite_name(clstats.max_total_power_sat),
           satfire_sector_name(clstats.max_total_power_sector), start_str, end_str,
           clstats.max_total_power / 100.0);

    ctime_r(&clstats.min_total_power_start, start_str);
    ctime_r(&clstats.min_total_power_end, end_str);

    printf("\n\n"
           "Min Image Power Stats:\n"
           "            satellite: %s\n"
           "               sector: %s\n"
           "                start: %s"
           "                  end: %s"
           "      Min Total Power: %.0lf MW\n\n",
           satfire_satellite_name(clstats.min_total_power_sat),
           satfire_sector_name(clstats.min_total_power_sector), start_str, end_str,
           clstats.min_total_power);

    ctime_r(&clstats.max_num_clusters_start, start_str);
    ctime_r(&clstats.max_num_clusters_end, end_str);

    printf("\n\n"
           "Max Image Number Clusters:\n"
           "                satellite: %s\n"
           "                   sector: %s\n"
           "                    start: %s"
           "                      end: %s"
           "           Total Clusters: %u\n\n",
           satfire_satellite_name(clstats.max_num_clusters_sat),
           satfire_sector_name(clstats.max_num_clusters_sector), start_str, end_str,
           clstats.max_num_clusters);

    ctime_r(&clstats.min_num_clusters_start, start_str);
    ctime_r(&clstats.min_num_clusters_end, end_str);

    printf("\n\n"
           "Min Image Number Clusters:\n"
           "                satellite: %s\n"
           "                   sector: %s\n"
           "                    start: %s"
           "                      end: %s"
           "           Total Clusters: %u\n\n",
           satfire_satellite_name(clstats.min_num_clusters_sat),
           satfire_sector_name(clstats.min_num_clusters_sector), start_str, end_str,
           clstats.min_num_clusters);
}

/*-------------------------------------------------------------------------------------------------
 *                             Steps in the processing pipeline.
 *-----------------------------------------------------------------------------------------------*/
struct PipelineLink {
    Courier *from;
    Courier *to;
};

static void *
directory_walker(void *arg)
{
    static char const threadname[] = "findfire-walker";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    struct DirWalkState dir_walk_state = dir_walk_new_with_root(options.data_dir);
    char const *path = dir_walk_next_path(&dir_walk_state);

    // The date of the most recent file process in the database.
    struct tm most_recent[SATFIRE_SATELLITE_NUM][SATFIRE_SECTOR_NUM] = {0};
    if (options.only_new) {
        int rc = 0;
        SFDatabaseH db = satfire_db_connect(options.database_file);

        for (unsigned int sat_entry = 0; sat_entry < SATFIRE_SATELLITE_NUM; ++sat_entry) {
            for (unsigned int sector_entry = 0; sector_entry < SATFIRE_SECTOR_NUM; ++sector_entry) {

                time_t ts = satfire_cluster_db_newest_scan_start(db, sat_entry, sector_entry);
                struct tm *res = gmtime_r(&ts, &most_recent[sat_entry][sector_entry]);
                Stopif(!res, break, "Error converting time stamp.");

                if (options.verbose) {
                    char buf[32] = {0};
                    fprintf(stdout, "    Latest: %s %s %s", satfire_satellite_name(sat_entry),
                            satfire_sector_name(sector_entry), asctime_r(res, buf));
                }
            }
        }

        rc = satfire_db_close(&db);
        Stopif(rc, goto CLEAN_UP_DIR_WALK_AND_RETURN, "Error querying cluster database.");

        dir_walk_set_directory_filter(&dir_walk_state, standard_dir_filter, most_recent);
    }

    Courier *to_filter = arg;
    courier_register_sender(to_filter);
    courier_wait_until_ready_to_send(to_filter);

    while (path) {

        char *owned_path = 0;
        asprintf(&owned_path, "%s", path);
        bool success = courier_send(to_filter, owned_path);

        Stopif(!success, break, "Failed to send to filter.");

        path = dir_walk_next_path(&dir_walk_state);
    }

    courier_done_sending(to_filter);

CLEAN_UP_DIR_WALK_AND_RETURN:
    dir_walk_destroy(&dir_walk_state);

    return 0;
}

static void *
path_filter(void *arg)
{
    static char const threadname[] = "findfire-filter";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    struct PipelineLink *links = arg;
    Courier *from_dir_walker = links->from;
    Courier *to_satfire_cluster_list_loader = links->to;

    SFDatabaseH db = 0;
    db = satfire_db_connect(options.database_file);
    Stopif(!db, exit(EXIT_FAILURE), "Error opening database.");

    SFClusterDatabaseQueryPresentH present_query = 0;
    present_query = satfire_cluster_db_prepare_to_query_present(db);
    Stopif(!present_query, exit(EXIT_FAILURE), "Error preparing query.");

    courier_register_receiver(from_dir_walker);
    courier_register_sender(to_satfire_cluster_list_loader);

    courier_wait_until_ready_to_receive(from_dir_walker);
    courier_wait_until_ready_to_send(to_satfire_cluster_list_loader);

    void *item = 0;
    while ((item = courier_receive(from_dir_walker))) {
        char *path = item;

        if (!skip_path(path, present_query)) {

            if (options.verbose) {
                printf("Processing: %s\n", path);
            }

            bool success = courier_send(to_satfire_cluster_list_loader, path);

            Stopif(!success, break, "Failed to send to loader.");
        } else {
            free(path);
        }
    }

    courier_done_receiving(from_dir_walker);
    courier_done_sending(to_satfire_cluster_list_loader);
    satfire_cluster_db_finalize_query_present(&present_query);
    satfire_db_close(&db);

    return 0;
}

static bool
is_cluster_a_keeper(struct SFCluster *clust)
{
    assert(clust);

    // Check if it meets our mask criteria
    bool keep_mask_criteria = false;
    struct SFPixelList const *pixels = satfire_cluster_pixels(clust);
    for (size_t i = 0; i < pixels->len; ++i) {
        short mask_flag = pixels->pixels[i].mask_flag;

        switch (mask_flag) {
        // Fallthrough is intentional
        case 10: // good_fire_pixel
        case 11: // saturated_fire_pixel
        case 12: // cloud_contaminated_fire_pixel
        case 13: // high_probability_fire_pixel
        case 14: // medium_probability_fire_pixel

        case 30: // temporally_filtered_good_fire_pixel
        case 31: // temporally_filtered_saturated_fire_pixel
        case 32: // temporally_filtered_cloud_contaminated_fire_pixel
        case 33: // temporally_filtered_high_probability_fire_pixel
        case 34: // temporally_filtered_medium_probability_fire_pixel
            keep_mask_criteria = true;
        }

        if (keep_mask_criteria) {
            break;
        }
    }

    return keep_mask_criteria;
}

static void *
fire_satfire_cluster_list_loader(void *arg)
{
    static char const threadname[] = "findfire-loader";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    struct PipelineLink *links = arg;
    Courier *from_filter = links->from;
    Courier *to_database = links->to;

    courier_register_receiver(from_filter);
    courier_register_sender(to_database);

    courier_wait_until_ready_to_receive(from_filter);
    courier_wait_until_ready_to_send(to_database);

    void *item = 0;
    while ((item = courier_receive(from_filter))) {
        char *path = item;

        bool success_sending = true;

        struct SFClusterList *clusters = satfire_cluster_list_from_file(path);
        if (!satfire_cluster_list_error(clusters)) {
            clusters = satfire_cluster_list_filter(clusters, is_cluster_a_keeper);

            success_sending = courier_send(to_database, clusters);
        } else {
            fprintf(stderr, "    Error processing file: %s\n", path);
            satfire_cluster_list_destroy(&clusters);
        }

        free(path);

        Stopif(!success_sending, break, "Failed to send to database.");
    }

    courier_done_receiving(from_filter);
    courier_done_sending(to_database);

    return 0;
}

static void *
database_filler(void *arg)
{
    static char const threadname[] = "findfire-dbase";
    static_assert(sizeof(threadname) <= 16, "threadname too long for OS");
    pthread_setname(threadname);

    Courier *from_satfire_cluster_list_loader = arg;
    courier_register_receiver(from_satfire_cluster_list_loader);
    courier_wait_until_ready_to_receive(from_satfire_cluster_list_loader);

    SFDatabaseH db = 0;
    SFClusterDatabaseAddH add_stmt = 0;

    db = satfire_db_connect(options.database_file);
    Stopif(!db, goto CLEANUP_AND_RETURN, "Error opening database.");

    add_stmt = satfire_cluster_db_prepare_to_add(db);
    Stopif(!add_stmt, goto CLEANUP_AND_RETURN, "Error preparing add statement.");

    // Stats on individual clusters.
    struct ClusterStats satfire_cluster_stats = satfire_cluster_stats_new();

    // Stats about satellite images.
    struct SFClusterListStats clstats = satfire_cluster_list_stats_new();

    void *item;
    while ((item = courier_receive(from_satfire_cluster_list_loader))) {
        struct SFClusterList *clusters = item;

        // Filter out clusters on the limb for some QC
        clusters = satfire_cluster_list_filter_scan_angle(clusters, MAX_SCAN_ANGLE);

        int failure = satfire_cluster_db_add(add_stmt, clusters);
        Stopif(failure, goto CLEANUP_AND_RETURN, "Error adding row to database.");

        enum SFSatellite sat = satfire_cluster_list_satellite(clusters);
        enum SFSector sector = satfire_cluster_list_sector(clusters);
        time_t start = satfire_cluster_list_scan_start(clusters);
        time_t end = satfire_cluster_list_scan_end(clusters);
        GArray *clusters_array = satfire_cluster_list_clusters(clusters);

        for (unsigned int i = 0; i < clusters_array->len; ++i) {

            struct SFCluster *curr_clust = g_array_index(clusters_array, struct SFCluster *, i);

            satfire_cluster_stats = satfire_cluster_stats_update(satfire_cluster_stats, sat, sector,
                                                                 start, end, curr_clust);
        }

        clstats = satfire_cluster_list_stats_update(clstats, clusters);

        satfire_cluster_list_destroy(&clusters);
    }

    if (options.verbose) {
        satfire_cluster_stats_print(satfire_cluster_stats);
        satfire_cluster_list_stats_print(clstats);
    }

    save_satfire_cluster_kml(
        satfire_cluster_stats.biggest_fire, satfire_cluster_stats.biggest_start,
        satfire_cluster_stats.biggest_end, satfire_cluster_stats.biggest_sat,
        satfire_cluster_stats.biggest_sector, satfire_cluster_stats.hottest_fire,
        satfire_cluster_stats.hottest_start, satfire_cluster_stats.hottest_end,
        satfire_cluster_stats.hottest_sat, satfire_cluster_stats.hottest_sector);

    satfire_cluster_stats_destroy(&satfire_cluster_stats);

    satfire_cluster_list_stats_destroy(&clstats);

CLEANUP_AND_RETURN:
    courier_done_receiving(from_satfire_cluster_list_loader);
    satfire_cluster_db_finalize_add(&add_stmt);
    satfire_db_close(&db);
    return 0;
}

/*-------------------------------------------------------------------------------------------------
 *                                             MAIN
 *-----------------------------------------------------------------------------------------------*/
static void
generic_destroy_satfire_cluster_list(void *cl)
{
    struct SFClusterList *list = cl;
    satfire_cluster_list_destroy(&list);
}

int
main(int argc, char *argv[argc + 1])
{
    int rc = EXIT_FAILURE;
    program_initialization(&argc, &argv);

    Courier dir_walk = courier_new();
    Courier filter = courier_new();
    Courier satfire_cluster_loader = courier_new();
    struct PipelineLink dir_walk_filter_link = {.from = &dir_walk, .to = &filter};
    struct PipelineLink filter_to_loader = {.from = &filter, .to = &satfire_cluster_loader};

    pthread_t threads[7] = {0};

    int s = pthread_create(&threads[0], 0, directory_walker, &dir_walk);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "directory_walker");

    s = pthread_create(&threads[1], 0, path_filter, &dir_walk_filter_link);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "path_filter");

    for (unsigned int i = 2; i < 6; ++i) {
        s = pthread_create(&threads[i], 0, fire_satfire_cluster_list_loader, &filter_to_loader);
        Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s(%u) thread.",
               "fire_satfire_cluster_list_loader", i);
    }

    s = pthread_create(&threads[6], 0, database_filler, &satfire_cluster_loader);
    Stopif(s, goto CLEANUP_AND_EXIT, "Error creating %s thread.", "database_filler");

    rc = EXIT_SUCCESS;

CLEANUP_AND_EXIT:

    for (unsigned int i = 0; i < sizeof(threads) / sizeof(threads[0]); ++i) {
        if (threads[i]) {
            s = pthread_join(threads[i], 0);
            if (s) {
                fprintf(stderr, "Error joining thread %u\n", i);
                rc = EXIT_FAILURE;
            }
        }
    }

    courier_destroy(&satfire_cluster_loader, generic_destroy_satfire_cluster_list);
    courier_destroy(&filter, free);
    courier_destroy(&dir_walk, free);

    program_finalization();

    return rc;
}
*/
