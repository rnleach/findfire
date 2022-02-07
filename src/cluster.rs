use crate::{
    pixel::PixelList,
    satellite::{Satellite, Sector},
};
use chrono::NaiveDateTime;

/** Represents a spatially contiguous cluster of [Pixel](crate::Pixel) objects.
 *
 * This also contains the aggregate properties of the cluster such as the total power of all the
 * constituent Pixels, the total area, and the temperature of the hottest Pixel in the cluster.
 *
 * It should be noted that a power, area, and temperature are not analyzed for every Pixel, so
 * the aggregate properties only aggregate these paramters for the Pixels that report values.
 */
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    power: f64,
    /// Total (sum) of the fire area of the points in the cluster with area in square meters.
    area: f64,
    /// Maximum temperature of all the pixels in the cluster in Kelvin.
    max_temp: f64,
    /// The maximum scan angle of any point in this cluster
    max_scan_angle: f64,
    /// Pixels making up the cluster.
    pixels: PixelList,
}

impl Default for Cluster {
    fn default() -> Self {
        Cluster {
            power: 0.0,
            area: 0.0,
            max_temp: 0.0,
            max_scan_angle: 0.0,
            pixels: PixelList::new(),
        }
    }
}

impl Cluster {
    /** Create a new Cluster with already initialized values. */
    pub fn new(
        power: f64,
        area: f64,
        max_temp: f64,
        max_scan_angle: f64,
        pixels: PixelList,
    ) -> Cluster {
        Cluster {
            power,
            area,
            max_temp,
            max_scan_angle,
            pixels,
        }
    }

    /** Get the total power of all pixels in the Cluster, megawatts. */
    pub fn total_power(&self) -> f64 {
        self.power
    }

    /** Get the total fire area of all pixels in the Cluster that had an area in the file, square
     * meters. */
    pub fn total_area(&self) -> f64 {
        self.area
    }

    /** Get the max fire temperature of all pixels in the Cluster that had a temperature in the file,
     * Kelvin. */
    pub fn max_temperature(&self) -> f64 {
        self.max_temp
    }

    /** Get the max scan angle of any pixel in this cluster. */
    pub fn max_scan_angle(&self) -> f64 {
        self.max_scan_angle
    }

    /** Get the number of SFPixels in a Cluster. */
    pub fn pixel_count(&self) -> usize {
        self.pixels.len()
    }

    /** Get access to the pixels in the cluster. */
    pub fn pixels(&self) -> &PixelList {
        &self.pixels
    }

    /*
    /// Add a fire point to this Cluster.
    fn add_fire_point(&mut self, fire_point: FirePoint)
    {
        assert(cluster);
        assert(fire_point);

        cluster->pixels = satfire_pixel_list_append(cluster->pixels, &fire_point->pixel);
        if (!isinf(fire_point->pixel.power)) {
            cluster->power += fire_point->pixel.power;
        }

        if (!isinf(fire_point->pixel.temperature)) {
            cluster->max_temp = fmax(cluster->max_temp, fire_point->pixel.temperature);
        }

        if (!isinf(fire_point->pixel.area)) {
            cluster->area += fire_point->pixel.area;
        }

        cluster->max_scan_angle = fmax(cluster->max_scan_angle, fire_point->pixel.scan_angle);
    }
    */
}

/*
/** Get the centroid of a cluster. */
struct SFCoord satfire_cluster_centroid(struct SFCluster const *cluster);

struct SFCoord
satfire_cluster_centroid(struct SFCluster const *cluster)
{
    assert(cluster);
    return satfire_pixel_list_centroid(cluster->pixels);
}

*/

/** A collection of [Cluster](crate::Cluster) objects.
 *
 * This collection stores a list of Clusters that are related. Specifically, they all come from
 * the same scan, or image. That means they share common scan start and end times, they come from
 * the same satellite, and they come from the same scan sector.
 */
pub struct ClusterList {
    satellite: Satellite,
    sector: Sector,
    /// Start time of the scan.
    start: NaiveDateTime,
    /// End time of the scan
    end: NaiveDateTime,
    /// List of [Cluster] objects associated with the above metadata.
    clusters: Vec<Cluster>,
}

/*
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
                                               ClusterList
-------------------------------------------------------------------------------------------------*/
void
satfire_cluster_list_destroy(struct SFClusterList **list)
{
    assert(list);
    assert(*list);

    if ((*list)->clusters) {
        g_array_unref((*list)->clusters);
    }

    // These are static strings!
    // if ((*list)->err_msg) {
    //    free((*list)->err_msg);
    //}

    free(*list);
    *list = 0;
}

enum SFSector
satfire_cluster_list_sector(struct SFClusterList *list)
{
    assert(list);
    return list->sector;
}

enum SFSatellite
satfire_cluster_list_satellite(struct SFClusterList *list)
{
    assert(list);
    return list->satellite;
}

time_t
satfire_cluster_list_scan_start(struct SFClusterList *list)
{
    assert(list);
    return list->start;
}

time_t
satfire_cluster_list_scan_end(struct SFClusterList *list)
{
    assert(list);
    return list->end;
}

bool
satfire_cluster_list_error(struct SFClusterList *list)
{
    assert(list);
    return list->error;
}

const char *
satfire_cluster_list_error_msg(struct SFClusterList *list)
{
    assert(list);
    return list->err_msg;
}

GArray *
satfire_cluster_list_clusters(struct SFClusterList *list)
{
    assert(list);

    if (list->error) {
        assert(!list->clusters); // Force to be NULL
    }

    return list->clusters;
}

struct SFClusterList *
satfire_cluster_list_filter_box(struct SFClusterList *list, struct SFBoundingBox box)
{
    assert(list);

    GArray *clusters = list->clusters;

    for (unsigned int i = 0; i < clusters->len; ++i) {
        struct SFCluster *clust = g_array_index(clusters, struct SFCluster *, i);

        struct SFCoord centroid = satfire_cluster_centroid(clust);

        if (!satfire_bounding_box_contains_coord(box, centroid, 0.0)) {
            clusters = g_array_remove_index_fast(clusters, i);
            --i; // Decrement this so we inspect this index again since a new value is there.
        }
    }

    list->clusters = clusters; // In case g_array_remove_index_fast() moved the array.

    return list;
}

struct SFClusterList *
satfire_cluster_list_filter_scan_angle(struct SFClusterList *list, double max_scan_angle)
{
    assert(list);

    GArray *clusters = list->clusters;

    for (unsigned int i = 0; i < clusters->len; ++i) {
        struct SFCluster *clust = g_array_index(clusters, struct SFCluster *, i);

        double clust_max_scan_angle = satfire_cluster_max_scan_angle(clust);

        if (clust_max_scan_angle >= max_scan_angle) {
            clusters = g_array_remove_index_fast(clusters, i);
            --i; // Decrement this so we inspect this index again since a new value is there.
        }
    }

    list->clusters = clusters; // In case g_array_remove_index_fast() moved the array.

    return list;
}

struct SFClusterList *
satfire_cluster_list_filter(struct SFClusterList *list, bool (*filter)(struct SFCluster *clust))
{

    assert(list);
    assert(filter);

    GArray *clusters = list->clusters;

    for (unsigned int i = 0; i < clusters->len; ++i) {
        struct SFCluster *clust = g_array_index(clusters, struct SFCluster *, i);

        if (!filter(clust)) {
            clusters = g_array_remove_index_fast(clusters, i);
            --i; // Decrement this so we inspect this index again since a new value is there.
        }
    }

    list->clusters = clusters; // In case g_array_remove_index_fast() moved the array.

    return list;
}

char const *
satfire_cluster_find_start_time(char const *fname)
{
    char const *start = strstr(fname, "_s");
    if (start)
        return start + 2;
    return start;
}

char const *
satfire_cluster_find_end_time(char const *fname)
{
    char const *end = strstr(fname, "_e");
    if (end)
        return end + 2;
    return end;
}

static void
local_satfire_cluster_destroy(void *cluster)
{
    struct SFCluster **clst = cluster;
    satfire_cluster_destroy(clst);
}

static GArray *
clusters_from_fire_points(GArray const *points)
{
    GArray *clusters = g_array_sized_new(false, true, sizeof(struct SFCluster *), 100);
    g_array_set_clear_func(clusters, local_satfire_cluster_destroy);

    GArray *satfire_cluster_points = g_array_sized_new(false, true, sizeof(struct FirePoint), 20);

    for (unsigned int i = 0; i < points->len; i++) {

        struct FirePoint *fp = &g_array_index(points, struct FirePoint, i);

        if (fp->x == 0 && fp->y == 0)
            continue;

        satfire_cluster_points = g_array_append_val(satfire_cluster_points, *fp);
        fp->x = 0;
        fp->y = 0;

        for (unsigned int j = i + 1; j < points->len; j++) {
            struct FirePoint *candidate = &g_array_index(points, struct FirePoint, j);

            if (candidate->x == 0 && candidate->y == 0)
                continue;
            for (unsigned int k = 0; k < satfire_cluster_points->len; ++k) {
                struct FirePoint *a_point_in_cluster =
                    &g_array_index(satfire_cluster_points, struct FirePoint, k);

                int dx = abs(a_point_in_cluster->x - candidate->x);
                int dy = abs(a_point_in_cluster->y - candidate->y);

                if (dx <= 1 && dy <= 1) {
                    satfire_cluster_points = g_array_append_val(satfire_cluster_points, *candidate);
                    candidate->x = 0;
                    candidate->y = 0;
                }
            }
        }

        struct SFCluster *curr_clust = satfire_cluster_new();
        struct FirePoint *curr_fire_point =
            &g_array_index(satfire_cluster_points, struct FirePoint, 0);
        satfire_cluster_add_fire_point(curr_clust, curr_fire_point);

        for (unsigned int j = 1; j < satfire_cluster_points->len; ++j) {

            curr_fire_point = &g_array_index(satfire_cluster_points, struct FirePoint, j);
            satfire_cluster_add_fire_point(curr_clust, curr_fire_point);
        }

        clusters = g_array_append_val(clusters, curr_clust);
        satfire_cluster_points = g_array_set_size(satfire_cluster_points, 0);
    }

    g_array_unref(satfire_cluster_points);

    return clusters;
}

struct SFClusterList *
satfire_cluster_list_from_file(char const *full_path)
{
    struct SFClusterList *clist = calloc(1, sizeof(struct SFClusterList));
    char *err_msg = 0;
    GArray *points = 0;
    GArray *clusters = 0;

    char const *fname = get_file_name(full_path);

    // Get the satellite
    enum SFSatellite satellite = satfire_satellite_string_contains_satellite(fname);
    err_msg = "Error parsing satellite name";
    Stopif(satellite == SATFIRE_SATELLITE_NONE, goto ERR_RETURN, "Error parsing satellite name");
    clist->satellite = satellite;

    // Get the sector name
    enum SFSector sector = satfire_sector_string_contains_sector(fname);
    err_msg = "Error parsing sector name";
    Stopif(sector == SATFIRE_SECTOR_NONE, goto ERR_RETURN, "Error parsing sector name");
    clist->sector = sector;

    // Get the start and end times
    clist->start = parse_time_string(satfire_cluster_find_start_time(fname));
    clist->end = parse_time_string(satfire_cluster_find_end_time(fname));

    // Get the clusters member.
    struct SatFireImage fdata = {0};
    bool ok = fire_sat_image_open(full_path, &fdata);
    Stopif(!ok, err_msg = "Error opening NetCDF file";
           goto ERR_RETURN, "Error opening NetCDF file %s", full_path);

    points = fire_sat_image_extract_fire_points(&fdata);
    fire_sat_image_close(&fdata);
    Stopif(!points, goto ERR_RETURN, "Error extracting fire points.");

    clusters = clusters_from_fire_points(points);
    Stopif(!clusters, err_msg = "Error creating clusters.";
           goto ERR_RETURN, "Error creating clusters from fire points.");
    g_array_unref(points);

    clist->clusters = clusters;

    return clist;

ERR_RETURN:

    if (points) {
        g_array_unref(points);
        points = 0;
    }

    if (clusters) {
        g_array_unref(clusters);
        clusters = 0;
    }

    clist->error = true;
    clist->err_msg = err_msg;
    return clist;
}

unsigned int
satfire_cluster_list_length(struct SFClusterList *list)
{
    assert(list);
    return list->clusters->len;
}

double
satfire_cluster_list_total_power(struct SFClusterList *list)
{
    assert(list);

    double sum = 0.0;
    for (unsigned int i = 0; i < list->clusters->len; i++) {
        sum += g_array_index(list->clusters, struct SFCluster *, i)->power;
    }

    return sum;
}
*/
