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

struct SFWildfire *
satfire_wildfire_new(unsigned int id, time_t first_observed, time_t last_observed,
                     struct SFClusterRow *initial)
{
    struct SFWildfire *new = malloc(sizeof(struct SFWildfire));
    assert(new);

    *new = (struct SFWildfire){
        .id = id,
        .first_observed = first_observed,
        .last_observed = last_observed,
        .centroid = satfire_cluster_db_satfire_cluster_row_centroid(initial),
        .max_power = satfire_cluster_db_satfire_cluster_row_power(initial),
        .max_temperature = satfire_cluster_db_satfire_cluster_row_max_temperature(initial),
        .sat = satfire_cluster_db_satfire_cluster_row_satellite(initial),
        .area = satfire_cluster_db_satfire_cluster_row_steal_pixels(initial),
    };

    satfire_cluster_db_satfire_cluster_row_finalize(initial);

    return new;
}

void
satfire_wildfire_destroy(struct SFWildfire **wildfire)
{
    if (*wildfire) {
        satfire_pixel_list_destroy((*wildfire)->area);
        free(*wildfire);
        wildfire = 0;
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
satfire_wildfire_update(struct SFWildfire *wildfire, struct SFClusterRow *row)
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

    struct SFPixelList *wf_pixels = wildfire->area;
    struct SFPixelList const *row_pixels = satfire_cluster_db_satfire_cluster_row_pixels(row);
    max_merge_pixel_lists(&wf_pixels, row_pixels);

    wildfire->centroid = satfire_pixel_list_centroid(wildfire->area);
    wildfire->area = wf_pixels;

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
        free(list->fires);
        free(list);
    }

    return 0;
}

struct SFWildfireList *
satfire_wildfirelist_add_fire(struct SFWildfireList *list, struct SFWildfire *new_fire)
{
    if (!list || list->len + 1 < list->capacity) {
        list = expand_wildfirelist(list, 1);
    }

    list->fires[list->len] = *new_fire;
    list->len++;
    return list;
}

struct SFClusterRow *
satfire_wildfirelist_take_update(struct SFWildfireList *const list, struct SFClusterRow *row)
{
    if (!list) {
        return row;
    }

    struct SFPixelList const *cluster_pixels = satfire_cluster_db_satfire_cluster_row_pixels(row);
    for (unsigned int i = 0; i < list->len; ++i) {
        struct SFPixelList *fire_pixels = list->fires[i].area;
        if (satfire_pixel_lists_adjacent_or_overlap(fire_pixels, cluster_pixels, 1.0e-5)) {
            satfire_wildfire_update(&list->fires[i], g_steal_pointer(&row));
            return 0;
        }
    }

    return row;
}

struct SFWildfireList *
satfire_wildfirelist_extend(struct SFWildfireList *list, struct SFWildfireList *const src)
{
    if (!src) {
        return list;
    }

    size_t need_cap = (list ? list->capacity : 0) + src->len;
    if (list->capacity < need_cap) {
        list = expand_wildfirelist(list, src->len);
    }

    for (unsigned int i = 0; i < src->len; ++i) {
        list = satfire_wildfirelist_add_fire(list, &src->fires[i]);
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
                merged_away = satfire_wildfirelist_add_fire(merged_away, &list->fires[j]);

                // Move the fire in the last position here - and zero out the value at the end of
                // the list
                list->fires[j] = list->fires[list->len - 1];
                memset(&list->fires[list->len - 1], 0, sizeof(struct SFWildfire));

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

struct SFWildfireList *
satfire_wildfirelist_drain_fires_not_seen_since(struct SFWildfireList *const list,
                                                struct SFWildfireList *tgt_list, time_t older_than)
{
    // TODO
    assert(false);
    return 0;
}
