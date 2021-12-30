#include "satfire.h"

#include <assert.h>
#include <stdbool.h>
#include <string.h>

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
    // TODO implement
    assert(false);
    return 0;
}

void
satfire_wildfire_destroy(struct SFWildfire **wildfire)
{
    // TODO implement
    assert(false);
    return;
}

unsigned int
satfire_wildfire_id(struct SFWildfire const *wildfire)
{
    // TODO implement
    assert(false);
    return 0;
}

time_t
satfire_wildfire_get_first_observed(struct SFWildfire const *wildfire)
{
    // TODO implement
    assert(false);
    return 0;
}

time_t
satfire_wildfire_get_last_observed(struct SFWildfire const *wildfire)
{
    // TODO implement
    assert(false);
    return 0;
}

struct SFCoord
satfire_wildfire_centroid(struct SFWildfire const *wildfire)
{
    // TODO implement
    assert(false);
    return (struct SFCoord){0};
}

double
satfire_wildfire_max_power(struct SFWildfire const *wildfire)
{
    // TODO implement
    assert(false);
    return 0;
}

double
satfire_wildfire_max_temperature(struct SFWildfire const *wildfire)
{
    // TODO implement
    assert(false);
    return 0;
}

const struct SFPixelList *
satfire_wildfire_pixels(struct SFWildfire const *wildfire)
{
    // TODO implement
    assert(false);
    return 0;
}

enum SFSatellite
satfire_wildfire_satllite(struct SFWildfire const *wildfire)
{
    // TODO implement
    assert(false);
    return 0;
}

void
satfire_wildfire_update(struct SFWildfire *wildfire, struct SFCluster *cluster)
{
    // TODO implement
    assert(false);
    return;
}

/*-------------------------------------------------------------------------------------------------
 *                                        Wildfire List
 *-----------------------------------------------------------------------------------------------*/
struct SFWildfireList {
};

struct SFWildfireList *
satfire_wildfirelist_destroy(struct SFWildfireList *list)
{
    // TODO
    assert(false);
    return 0;
}

struct SFWildfireList *
satfire_wildfirelist_add_fire(struct SFWildfireList *list, struct SFWildfire *new_fire)
{
    // TODO
    assert(false);
    return 0;
}

struct SFClusterRow *
satfire_wildfirelist_take_update(struct SFWildfireList *const list, struct SFClusterRow *clust)
{
    // TODO
    assert(false);
    return 0;
}

struct SFWildfireList *
satfire_wildfirelist_extend(struct SFWildfireList *list, struct SFWildfireList *const src)
{
    // TODO
    assert(false);
    return 0;
}

struct SFWildfireList *
satfire_wildfirelist_merge_fires(struct SFWildfireList *const list,
                                 struct SFWildfireList *merged_away)
{
    // TODO
    assert(false);
    return 0;
}

struct SFWildfireList *
satfire_wildfirelist_drain_fires_not_seen_since(struct SFWildfireList *const list,
                                                struct SFWildfireList *tgt_list, time_t older_than)
{
    // TODO
    assert(false);
    return 0;
}
