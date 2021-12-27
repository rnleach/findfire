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
satfire_wildfire_new(unsigned int id, time_t observed)
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
