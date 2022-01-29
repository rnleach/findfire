use crate::{geo::Coord, pixel::PixelList, satellite::Satellite};
use chrono::NaiveDateTime;

/**
 * The aggregate properties of a temporally connected group of [Cluster] objects.
 *
 * While the [Cluster]s that make up a fire may come from any [Sector] of a satellite scan, they
 * must come from the same [Satellite] because of the difficulty associated with the different map
 * projections and parallax. Currently the geo-location of observed [Pixel]s does not take parallax
 * into account. While this is a neglibible issue for low elevation locations considering the
 * resolution of the satellites, for higher elevations it can cause a significant error. Also, for
 * each satellite, the data was reprojected into the exact same projection each time. So every
 * image from a given satellite has the exact same [Pixel] locations on the Earth's surface. As a
 * result, aggregating values for maximum power, area, or temperature is straight forward. If we
 * had to deal with [Pixel]s from different satellites that don't totally overlap, or only
 * partially overlap, it's not straightforward at all how to combine the properties of those
 * [Pixel]s into a common projection.
 */
pub struct Fire {
    /// The scan start time of the first [Cluster] where this fire was detected.
    first_observed: NaiveDateTime,
    /// The scan end time of the last [Cluster] where this fire was detected.
    last_observed: NaiveDateTime,
    /// The centroid of all the combined [Cluster]s that contributed to this fire.
    centroid: Coord,
    /// The power of the most powerful [Cluster] that was associated with this fire. Note that
    /// several clusters may be associated with a fire at any given scan time, but they might be
    /// spatially separated (e.g. on different ends of the original fire). The powers of those
    /// different [Cluster]s are NOT combined to come up with a total power for the time. This
    /// represents the single most powerful [Cluster] aggregated into this fire.
    max_power: f64,
    /// The maximum temperature of any [Pixel] that was ever associated with this fire.
    max_temperature: f64,
    /// An unique ID number for this fire that will be used identify this fire in a database that
    /// will also be used to associate this fire with [Cluster]s which are a part of it.
    id: u64,
    /// Each [Pixel] in this contains the maximum power, area, and temperature observed in it's
    /// area during the fire. Since all the data for each satellite is projected to a common grid
    /// before being published online, throughout the life of the fire the [Pixel]s will perfectly
    /// overlap. This is kind of a composite of the properties of the fire over it's lifetime.
    area: PixelList,
    /// The satellite the [Cluster]s that were a part of this fire were observed with.
    sat: Satellite,
}

/// A list of [Fire] objects.
pub struct FireList(Vec<Fire>);

/*

/*-------------------------------------------------------------------------------------------------
 *                                        Wildfire
 *-----------------------------------------------------------------------------------------------*/

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

#include "satfire.h"

#include <assert.h>
#include <stdbool.h>
#include <string.h>
#include <tgmath.h>

#include "sf_private.h"
#include "sf_util.h"

// Globally defined readonly constants. Mostly error messages.
extern char const *out_of_memory;

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

/** Remove fires from \p list that are likely no longer burning.
 *
 * \param list is the source list to drain fires from if they are older than \p older_than.
 * \param tgt_list is the list to add the drained elements into. If this point is \c NULL, then a
 * new list will be created. The return value of this function should be assigned to the variable
 * that was passed into this argument in case it was moved for a reallocation.
 * \param current_time is the current time of the clusters that are being processed.
 *
 * \returns an updated pointer to \p tgt_list.
 */
struct SFWildfireList *satfire_wildfirelist_drain_stale_fires(struct SFWildfireList *const list,
                                                              struct SFWildfireList *tgt_list,
                                                              time_t current_time);

/** Get the number of fires in the list. */
size_t satfire_wildfirelist_len(struct SFWildfireList const *list);

/** Get a reference to an element at a given index. */
struct SFWildfire const *satfire_wildfirelist_get(struct SFWildfireList const *list, size_t index);

/*-------------------------------------------------------------------------------------------------
                                               Wildfire
-------------------------------------------------------------------------------------------------*/
static struct SFWildfire
satfire_wildfire_initialize(unsigned int id, struct SFClusterRow *initial)
{
    return (struct SFWildfire){
        .id = id,
        .first_observed = satfire_cluster_db_satfire_cluster_row_start(initial),
        .last_observed = satfire_cluster_db_satfire_cluster_row_end(initial),
        .centroid = satfire_cluster_db_satfire_cluster_row_centroid(initial),
        .max_power = satfire_cluster_db_satfire_cluster_row_power(initial),
        .max_temperature = satfire_cluster_db_satfire_cluster_row_max_temperature(initial),
        .sat = satfire_cluster_db_satfire_cluster_row_satellite(initial),
        .area = satfire_cluster_db_satfire_cluster_row_steal_pixels(initial),
    };
}

struct SFWildfire *
satfire_wildfire_new(unsigned int id, struct SFClusterRow *initial)
{
    struct SFWildfire *new = malloc(sizeof(struct SFWildfire));
    assert(new);

    *new = satfire_wildfire_initialize(id, initial);

    return new;
}

struct SFWildfire *
satfire_wildfire_clone(struct SFWildfire const *src)
{
    struct SFWildfire *clone = malloc(sizeof(*src));

    *clone = *src;
    clone->area = satfire_pixel_list_copy(src->area);

    return clone;
}

void
satfire_wildfire_print(struct SFWildfire const *src)
{
    if (!src) {
        printf("NULL - no wildfire information.\n");
        return;
    }

    struct tm start = *gmtime(&src->first_observed);
    char start_buf[32] = {0};
    strftime(start_buf, sizeof(start_buf), "%Y-%m-%d %H:%M:%SZ", &start);

    struct tm end = *gmtime(&src->last_observed);
    char end_buf[32] = {0};
    strftime(end_buf, sizeof(end_buf), "%Y-%m-%d %H:%M:%SZ", &end);

    double duration = satfire_wildfire_duration(src);

    int days = (int)floor(duration / 60.0 / 60.0 / 24.0);
    double so_far = days * 60.0 * 60.0 * 24.0;

    int hours = (int)floor((duration - so_far) / 60.0 / 60.0);

    double max_scan_angle = satfire_pixel_list_max_scan_angle(src->area);

    printf("~~ Wildfire ~~\n");
    printf("                   id: %u\n", src->id);
    printf("            satellite: %s\n", satfire_satellite_name(src->sat));
    printf("       first observed: %s\n", start_buf);
    printf("        last observed: %s\n", end_buf);
    printf("             duration: %d days %d hours\n", days, hours);
    printf("          centered at: (%10.6lf, %11.6lf)\n", src->centroid.lat, src->centroid.lon);
    printf("           num pixels: %lu\n", src->area->len);
    printf("   maximum scan angle: %7.0lf degrees\n", max_scan_angle);
    printf("        maximum power: %7.0lf MW\n", src->max_power);
    printf("  maximum temperature: %7.0lf K\n", src->max_temperature);

    return;
}

static void
satfire_wildfire_cleanup(struct SFWildfire *wildfire)
{
    if (wildfire) {
        satfire_pixel_list_destroy(wildfire->area);
    }
}

void
satfire_wildfire_destroy(struct SFWildfire *wildfire)
{
    if (wildfire) {
        satfire_wildfire_cleanup(wildfire);
        free(wildfire);
    }

    return;
}

unsigned int
satfire_wildfire_id(struct SFWildfire const *wildfire)
{
    assert(wildfire);

    return wildfire->id;
}

time_t
satfire_wildfire_get_first_observed(struct SFWildfire const *wildfire)
{
    assert(wildfire);

    return wildfire->first_observed;
}

time_t
satfire_wildfire_get_last_observed(struct SFWildfire const *wildfire)
{
    assert(wildfire);

    return wildfire->last_observed;
}

double
satfire_wildfire_duration(struct SFWildfire const *wildfire)
{
    if (!wildfire) {
        return 0.0;
    }

    time_t start = wildfire->first_observed;
    time_t end = wildfire->last_observed;

    return difftime(end, start);
}

struct SFCoord
satfire_wildfire_centroid(struct SFWildfire const *wildfire)
{
    assert(wildfire);

    return wildfire->centroid;
}

double
satfire_wildfire_max_power(struct SFWildfire const *wildfire)
{
    assert(wildfire);

    return wildfire->max_power;
}

double
satfire_wildfire_max_temperature(struct SFWildfire const *wildfire)
{
    assert(wildfire);

    return wildfire->max_temperature;
}

const struct SFPixelList *
satfire_wildfire_pixels(struct SFWildfire const *wildfire)
{
    assert(wildfire);

    return wildfire->area;
}

enum SFSatellite
satfire_wildfire_satellite(struct SFWildfire const *wildfire)
{
    assert(wildfire);

    return wildfire->sat;
}

static void
max_merge_pixel_lists(struct SFPixelList **leftp, struct SFPixelList const *right)
{

    for (unsigned int rpi = 0; rpi < right->len; rpi++) {
        struct SFPixel const *rp = &right->pixels[rpi];

        struct SFPixelList *left = *leftp;

        bool is_new = true;
        for (unsigned int lpi = 0; lpi < left->len; lpi++) {
            struct SFPixel *lp = &left->pixels[lpi];
            if (satfire_pixels_approx_equal(lp, rp, 1.0e-5)) {

                lp->power = fmax(lp->power, rp->power);
                lp->temperature = fmax(lp->temperature, rp->temperature);
                lp->area = fmax(lp->area, rp->area);
                lp->mask_flag = lp->mask_flag < rp->mask_flag ? lp->mask_flag : rp->mask_flag;
                lp->data_quality_flag = lp->data_quality_flag < rp->data_quality_flag
                                            ? lp->data_quality_flag
                                            : rp->data_quality_flag;

                is_new = false;
                break;
            }
        }

        if (is_new) {
            *leftp = satfire_pixel_list_append(*leftp, rp);
        }
    }
}

void
satfire_wildfire_update(struct SFWildfire *wildfire, struct SFClusterRow const *row)
{
    assert(wildfire);
    assert(row);
    assert(satfire_cluster_db_satfire_cluster_row_satellite(row) == wildfire->sat);

    time_t cluster_end = satfire_cluster_db_satfire_cluster_row_end(row);
    double cluster_power = satfire_cluster_db_satfire_cluster_row_power(row);
    double cluster_maxt = satfire_cluster_db_satfire_cluster_row_max_temperature(row);

    wildfire->max_power = fmax(cluster_power, wildfire->max_power);
    wildfire->max_temperature = fmax(cluster_maxt, wildfire->max_temperature);
    wildfire->last_observed = cluster_end;

    struct SFPixelList const *row_pixels = satfire_cluster_db_satfire_cluster_row_pixels(row);
    max_merge_pixel_lists(&wildfire->area, row_pixels);

    wildfire->centroid = satfire_pixel_list_centroid(wildfire->area);

    return;
}

/*-------------------------------------------------------------------------------------------------
 *                                        Wildfire List
 *-----------------------------------------------------------------------------------------------*/
static struct SFWildfireList *
expand_wildfirelist(struct SFWildfireList *list, size_t increase_by_at_least)
{
    struct SFWildfireList *new = 0;

    if (!list) {
        new = malloc(sizeof(struct SFWildfireList));
        new->len = 0;
        new->capacity = 0;
        new->fires = 0;
    } else {
        new = list;
    }

    size_t new_cap = 3 * new->capacity / 2;
    if (new_cap < 12) {
        new_cap = 12;
    }

    size_t expansion = new_cap - new->capacity;

    if (expansion < increase_by_at_least) {
        new_cap += increase_by_at_least - expansion;
    }

    struct SFWildfire *new_fires = realloc(new->fires, new_cap * sizeof(struct SFWildfire));
    assert(new_fires);

    new->capacity = new_cap;
    new->fires = new_fires;

    return new;
}

struct SFWildfireList *
satfire_wildfirelist_destroy(struct SFWildfireList *list)
{
    if (list) {

        for (unsigned int i = 0; i < list->len; ++i) {
            satfire_wildfire_cleanup(&list->fires[i]);
        }

        free(list->fires);
        free(list);
    }

    return 0;
}

static struct SFWildfireList *
satfire_wildfirelist_steal_fire(struct SFWildfireList *list, struct SFWildfire *new_fire)
{
    if (!list || list->len + 1 > list->capacity) {
        list = expand_wildfirelist(list, 1);
    }

    // Move the fire to the list
    list->fires[list->len] = *new_fire;
    list->len++;

    // Wipe out the original
    memset(new_fire, 0, sizeof(*new_fire));

    return list;
}

struct SFWildfireList *
satfire_wildfirelist_add_fire(struct SFWildfireList *list, struct SFWildfire *new_fire)
{
    if (!list || list->len + 1 > list->capacity) {
        list = expand_wildfirelist(list, 1);
    }

    list->fires[list->len] = *new_fire;
    list->fires[list->len].area = satfire_pixel_list_copy(new_fire->area);
    list->len++;
    return list;
}

struct SFWildfireList *
satfire_wildfirelist_create_add_fire(struct SFWildfireList *list, unsigned int id,
                                     struct SFClusterRow *initial)
{
    if (!list || list->len + 1 > list->capacity) {
        list = expand_wildfirelist(list, 1);
    }

    list->fires[list->len] = satfire_wildfire_initialize(id, initial);
    list->len++;
    return list;
}

bool
satfire_wildfirelist_update(struct SFWildfireList *const list, struct SFClusterRow const *row)
{
    if (!list) {
        return false;
    }

    struct SFPixelList const *cluster_pixels = satfire_cluster_db_satfire_cluster_row_pixels(row);
    for (unsigned int i = 0; i < list->len; ++i) {
        struct SFPixelList *fire_pixels = list->fires[i].area;
        if (satfire_pixel_lists_adjacent_or_overlap(fire_pixels, cluster_pixels, 1.0e-5)) {
            satfire_wildfire_update(&list->fires[i], row);
            return true;
        }
    }

    return false;
}

struct SFWildfireList *
satfire_wildfirelist_extend(struct SFWildfireList *list, struct SFWildfireList *const src)
{
    if (!src) {
        return list;
    }

    size_t list_len = list ? list->len : 0;
    size_t list_cap = list ? list->capacity : 0;

    size_t need_cap = list_len + src->len;

    if (!list || list_cap < need_cap) {
        list = expand_wildfirelist(list, need_cap - list_cap);
    }

    for (unsigned int i = 0; i < src->len; ++i) {
        list = satfire_wildfirelist_steal_fire(list, &src->fires[i]);
    }

    src->len = 0;

    return list;
}

/** Merge two wildfires.
 *
 * Leave the larger fire in the \p left position and leave \p right with an unmodified version of
 * smaller fire.
 *
 * \returns a pointer to \p right, which may have been swapped with \p left.
 */
static struct SFWildfire *
merge_wildfires(struct SFWildfire *left, struct SFWildfire *right)
{
    assert(left);
    assert(right);
    assert(left->sat == right->sat);

    if (left->area->len < right->area->len) {
        // swap needed
        struct SFWildfire temp = {0};
        temp = *left;
        *left = *right;
        *right = temp;
    }

    if (right->first_observed < left->first_observed) {
        left->first_observed = right->first_observed;
    }

    if (right->last_observed > left->last_observed) {
        left->last_observed = right->last_observed;
    }

    // MUST DO THIS BEFORE UPDATING CENTROID
    max_merge_pixel_lists(&left->area, right->area);

    left->centroid = satfire_pixel_list_centroid(left->area);
    left->max_power = fmax(left->max_power, right->max_power);
    left->max_temperature = fmax(left->max_temperature, right->max_temperature);

    return right;
}

static bool
wildfires_overlap(struct SFWildfire const *left, struct SFWildfire const *right)
{
    assert(left);
    assert(right);
    assert(left->sat == right->sat);

    struct SFPixelList const *lp = left->area;
    struct SFPixelList const *rp = right->area;

    return satfire_pixel_lists_adjacent_or_overlap(lp, rp, 1.0e-5);
}

struct SFWildfireList *
satfire_wildfirelist_merge_fires(struct SFWildfireList *const list,
                                 struct SFWildfireList *merged_away)
{
    if (!list) {
        return merged_away;
    }

    for (unsigned int i = 0; i < list->len; ++i) {
        for (unsigned int j = i + 1; j < list->len; ++j) {
            if (wildfires_overlap(&list->fires[i], &list->fires[j])) {
                merge_wildfires(&list->fires[i], &list->fires[j]);
                merged_away = satfire_wildfirelist_steal_fire(merged_away, &list->fires[j]);

                // Move the fire in the last position here - 'j' has already been cleaned up by
                // the satfire_wildfirelist_steal_fire() function.
                list->fires[j] = list->fires[list->len - 1];

                // Shrink the list length since we moved the element to the other list
                list->len--;

                // Start over comparing to the rest of the list to make sure there are no more
                // overlaps
                j = i; // j will become i+1 again after it is incremented for the next iteration.
            }
        }
    }

    return merged_away;
}

static bool
wildfire_is_stale(struct SFWildfire *fire, time_t current_time)
{

    assert(fire);

    // Seconds in a four day duration
    double const four_days = 60 * 60 * 24 * 4;
    double const thirty_days = 60 * 60 * 24 * 30;

    time_t last_observed = satfire_wildfire_get_last_observed(fire);
    double duration_since_last_observed = difftime(current_time, last_observed);

    // Give it at least four days to come back to life again.
    if (duration_since_last_observed < four_days) {
        return false;
    }

    // If it's been out for 30 days, it's stale
    if (duration_since_last_observed > thirty_days) {
        return true;
    }

    double wildfire_duration = satfire_wildfire_duration(fire);

    // If it's not been seen in a longer time than it was burning, call it stale.
    if (wildfire_duration < duration_since_last_observed) {
        return true;
    }

    return false;
}

struct SFWildfireList *
satfire_wildfirelist_drain_stale_fires(struct SFWildfireList *const list,
                                       struct SFWildfireList *tgt_list, time_t current_time)
{
    if (!list) {
        return tgt_list;
    }

    for (unsigned int i = 0; i < list->len; ++i) {
        if (wildfire_is_stale(&list->fires[i], current_time)) {

            tgt_list = satfire_wildfirelist_steal_fire(tgt_list, &list->fires[i]);

            // Move the fire in the last position here - 'i' was cleand up by the function
            // satfire_wildfirelist_steal_fire()
            list->fires[i] = list->fires[list->len - 1];

            // Shrink the list length since we moved the element to the other list
            list->len--;

            i--; // So we'll look at this index again since we just moved something into it.
        }
    }

    return tgt_list;
}

size_t
satfire_wildfirelist_len(struct SFWildfireList const *list)
{
    return list ? list->len : 0;
}

struct SFWildfire const *
satfire_wildfirelist_get(struct SFWildfireList const *list, size_t index)
{
    return &list->fires[index];
}
*/
