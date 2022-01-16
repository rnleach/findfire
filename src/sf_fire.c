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
                                               Wildfire
-------------------------------------------------------------------------------------------------*/
struct SFWildfire {
    time_t first_observed;
    time_t last_observed;
    struct SFCoord centroid;
    double max_power;
    double max_temperature;
    unsigned int id;
    struct SFPixelList *area;
    enum SFSatellite sat;
};

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
struct SFWildfireList {
    size_t len;
    size_t capacity;
    struct SFWildfire *fires;
};

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
