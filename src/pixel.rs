use crate::geo::Coord;

/// The coordinates describing the area of a pixel viewed from a GOES satellite.
pub struct Pixel {
    /// The upper left (northwest) corner point of the pixel
    ul: Coord,
    /// The lower left (southwest) corner point of the pixel
    ll: Coord,
    /// The lower right (southeast) corner point of the pixel.
    lr: Coord,
    /// The upper right (northeast) corner point of the pixel.
    ur: Coord,
    /// The radiative power in MegaWatts in this pixel.
    power: f64,
    /// The estimated area of the pixel covered by the fire in square meters.
    area: f64,
    /// The estimated temperature of the fire in K
    temperature: f64,
    /// This is the scan angle as measured in the coordinate system of the satellite. The satellite
    /// measures the x and y positions of a pixel on a grid by the angle each makes with the central
    /// point which looks at nadir on the Earth. There are two values, an x scan angle and a y scan
    /// angle. They are combined via the Euclidian norm sqrt(x^2 + y^2) to form the scan_angle.
    ///
    /// Constant values of the scan angle form concentric circles around the nadir point on the
    /// Earth's surface. All points along that line have a very similar (equal if the Earth was a
    /// sphere) angle between the satellites view and the local zenith. This is a good proxy for
    /// how much of an edge on vs straight down view, which can be useful for quality control.
    scan_angle: f64,
    /// Mask is a code that describes the outcome of the algorithms that characterize a fire point.
    ///
    /// See the satfire_satellite_mask_code_to_string() function for reference.
    mask_flag: i16,
    /// Data Quality Flag
    ///
    /// See the satfire_satellite_dqf_code_to_string() function for reference.
    data_quality_flag: i16,
}

/// A pixel list stores a list of Pixel objects.
pub struct PixelList(Vec<Pixel>);

/*
/*-------------------------------------------------------------------------------------------------
 *                                          SFPixel
 *-----------------------------------------------------------------------------------------------*/
/** Calculate the centroid of a SFPixel.
 *
 * This function uses an algorithm that assumes the pixel is a quadrilateral, which is enforced
 * by the definition of the SFPixel type.
 */
struct SFCoord satfire_pixel_centroid(struct SFPixel const pxl[static 1]);

/** Tests if these pixels are basically the same pixel in a geographic sense (not including power).
 *
 * This compares the four corners of the pixel using the satfire_coord_are_close() function.
 */
bool satfire_pixels_approx_equal(struct SFPixel const left[static 1],
                                 struct SFPixel const right[static 1], double eps);

/** Determine if a coordinate is interior to a pixel.
 *
 * Interior means that it is NOT on the boundary. The eps parameter is used by an interanl line
 * intersection function to detect if the intersection point is very close to an end point.
 */
bool satfire_pixel_contains_coord(struct SFPixel const pxl[static 1], struct SFCoord coord,
                                  double eps);

/** Determine if satellite pixels overlap.
 *
 * Overlapping is defined as one pixel having a vertex / corner that is interior to the other one
 * or as pixels having edges that intersect.
 *
 * The eps parameter is used as a parameter for any cases where floating point values need to be
 * compared. There are a few places in the algorithm where that happens, and if they are within
 * eps units of each other, they are considered equal.
 */
bool satfire_pixels_overlap(struct SFPixel const left[static 1],
                            struct SFPixel const right[static 1], double eps);

/** Determine if satellite pixels are adjacent.
 *
 * Adjacent is defined as having at least one corner that is 'eps' close to a coordinate in the
 * other. Adjacent pixels may overlap also because satfire_pixels_overlap() uses the eps variable in
 * determining overlap. However, if there is a large overlap, the pixels aren't adjacent.
 *
 * \param left a satellite pixel to check.
 * \param right the pixel to check against.
 * \param eps The scale to use for comparison in the same units as the lat and lon.
 **/
bool satfire_pixels_are_adjacent(struct SFPixel const left[static 1],
                                 struct SFPixel const right[static 1], double eps);

/** Determine if satellite pixels are adjacent or overlapping.
 *
 * \param left a satellite pixel to check.
 * \param right the pixel to check against.
 * \param eps The scale to use for comparison in the same units as the lat and lon.
 **/
bool satfire_pixels_are_adjacent_or_overlap(struct SFPixel const left[static 1],
                                            struct SFPixel const right[static 1], double eps);

/*-------------------------------------------------------------------------------------------------
 *                                         SFPixelList
 *-----------------------------------------------------------------------------------------------*/
/** Create a new SFPixelList. */
struct SFPixelList *satfire_pixel_list_new();

/** Create a new SFPixelList with a given capacity. */
struct SFPixelList *satfire_pixel_list_new_with_capacity(size_t capacity);

/** Destroy a SFPixelList.  */
struct SFPixelList *satfire_pixel_list_destroy(struct SFPixelList plist[static 1]);

/** Create a deep copy of the SFPixelList. */
struct SFPixelList *satfire_pixel_list_copy(struct SFPixelList const *plist);

/** Append a SFPixel to the list.
 *
 * Reallocates the backing array if necessary and returns the new pointer, so always use the return
 * value as the new list. If the system is running out of memory and the allocation fails, it
 * aborts the program.
 *
 * \return A (potentially new) pointer to the list \param plist.
 */
struct SFPixelList *satfire_pixel_list_append(struct SFPixelList list[static 1],
                                              struct SFPixel const apix[static 1]);

/** Clear the list but keep the memory intact.
 *
 * After this call the list is basically in the same state as after calling
 * satfire_pixel_list_new().
 */
struct SFPixelList *satfire_pixel_list_clear(struct SFPixelList list[static 1]);

/** Calculate the centroid of a SFPixelList. */
struct SFCoord satfire_pixel_list_centroid(struct SFPixelList const list[static 1]);

/** Calculate the total power in a SFPixelList, megawatts. */
double satfire_pixel_list_total_power(struct SFPixelList const list[static 1]);

/** Calculate the total area in a SFPixelList, square meters. */
double satfire_pixel_list_total_area(struct SFPixelList const list[static 1]);

/** Calculate the maximum temperature in a SFPixelList, kelvin. */
double satfire_pixel_list_max_temperature(struct SFPixelList const list[static 1]);

/** Calculate the maximum scan angle in a SFPixelList, degrees. */
double satfire_pixel_list_max_scan_angle(struct SFPixelList const list[static 1]);

/** Check to see if these two \ref SFPixelList are adjacent or overlapping. */
bool satfire_pixel_lists_adjacent_or_overlap(struct SFPixelList const left[static 1],
                                             struct SFPixelList const right[static 1], double eps);

/** Get a bounding box for this list of pixels. */
struct SFBoundingBox satfire_pixel_list_bounding_box(struct SFPixelList const list[static 1]);
/*-------------------------------------------------------------------------------------------------
 *                                  Pixel List Binary Format
 *-----------------------------------------------------------------------------------------------*/
// The binary formatting is for storing or retrieving from a database.

/** Calculate the amount of space needed in a buffer to encode a SFPixelList into binary. */
size_t satfire_pixel_list_binary_serialize_buffer_size(struct SFPixelList const plist[static 1]);

/** Encode the SFPixelList into a binary format suitable for storing in a database.
 *
 * At this time it doesn't support a portable format, meaning no corrections are made for
 * endianness or any padding in the array.
 *
 * \return The number of bytes written.
 */
size_t satfire_pixel_list_binary_serialize(struct SFPixelList const plist[static 1],
                                           size_t buf_size, unsigned char buffer[buf_size]);

/** Deserialize an array of bytes into a SFPixelList.
 *
 * \return an allocated SFPixelList that should be cleaned up with satfire_pixel_list_destroy(). In
 * the event of an error, it returns NULL.
 */
struct SFPixelList *
satfire_pixel_list_binary_deserialize(unsigned char const buffer[static sizeof(size_t)]);

/*-------------------------------------------------------------------------------------------------
 *                                         KML Export
 *-----------------------------------------------------------------------------------------------*/
/** Write out a pixel list in KML format.
 *
 * This will print out a multigeometry KML element. It should be composed as part of a function
 * that outputs a KML file where that higher function adds style information and the rest of the
 * document.
 */
void satfire_pixel_list_kml_write(FILE *strm, struct SFPixelList const plist[static 1]);

/*-------------------------------------------------------------------------------------------------
 *                                         SatPixels
 *-----------------------------------------------------------------------------------------------*/

static struct SFBoundingBox
satfire_pixel_bounding_box(struct SFPixel const pxl[static 1])
{
    double xmax = fmax(pxl->ur.lon, pxl->lr.lon);
    double xmin = fmin(pxl->ul.lon, pxl->ll.lon);
    double ymax = fmax(pxl->ur.lat, pxl->ul.lat);
    double ymin = fmin(pxl->lr.lat, pxl->ll.lat);

    struct SFCoord ll = {.lat = ymin, .lon = xmin};
    struct SFCoord ur = {.lat = ymax, .lon = xmax};

    return (struct SFBoundingBox){.ll = ll, .ur = ur};
}

static bool
satfire_pixels_bounding_boxes_overlap(struct SFPixel const left[static 1],
                                      struct SFPixel const right[static 1], double eps)
{
    assert(left);
    assert(right);

    struct SFBoundingBox bb_left = satfire_pixel_bounding_box(left);
    struct SFBoundingBox bb_right = satfire_pixel_bounding_box(right);

    return bounding_boxes_overlap(bb_left, bb_right, eps);
}

struct SFCoord
satfire_pixel_centroid(struct SFPixel const pxl[static 1])
{
    /* Steps to calculatule the centroid of a quadrilateral.
     *
     *  1) Break the quadrilateral into two triangles by creating a diagonal.
     *  2) Calculate the centroid of each triangle by taking the average of it's 3 Coords
     *  3) Create a line connecting the centroids of each triangle.
     *  4) Repeat the process by creating the other diagonal.
     *  5) Find the intersection of the two resulting lines, that is the centroid of the
     *     quadrilateral.
     */

    struct SFCoord t1_c = triangle_centroid(pxl->ul, pxl->ll, pxl->lr);
    struct SFCoord t2_c = triangle_centroid(pxl->ul, pxl->ur, pxl->lr);
    struct Line diag1_centroids = {.start = t1_c, .end = t2_c};

    struct SFCoord t3_c = triangle_centroid(pxl->ul, pxl->ll, pxl->ur);
    struct SFCoord t4_c = triangle_centroid(pxl->lr, pxl->ur, pxl->ll);
    struct Line diag2_centroids = {.start = t3_c, .end = t4_c};

    struct IntersectResult res = lines_intersection(diag1_centroids, diag2_centroids, 1.0e-30);

    assert(res.does_intersect);

    return res.intersection;
}

bool
satfire_pixels_approx_equal(struct SFPixel const left[static 1],
                            struct SFPixel const right[static 1], double eps)
{
    return satfire_coord_are_close(left->ul, right->ul, eps) &&
           satfire_coord_are_close(left->ur, right->ur, eps) &&
           satfire_coord_are_close(left->lr, right->lr, eps) &&
           satfire_coord_are_close(left->ll, right->ll, eps);
}

bool
satfire_pixel_contains_coord(struct SFPixel const pxl[static 1], struct SFCoord coord, double eps)
{
    // Check if it's outside the bounding box first. This is easy, and if it is,
    // then we already know the answer.
    struct SFBoundingBox const box = satfire_pixel_bounding_box(pxl);

    if (!satfire_bounding_box_contains_coord(box, coord, eps)) {
        return false;
    }

    // Make a line from the point in question to each corner of the quadrilateral. If any of those
    // lines intersect an edge of the quadrilateral, the the point is outside. Note that the
    // line_intersection() function takes the eps argument and uses that to determine if the
    // intersection is near an end point. If it is, then we ignore it. So there is some fuzziness
    // to this function. If a coordinate outside the pixel is close enough to one of the edges,
    // it is possible it would be classified as inside. But it has to be eps close! And even then
    // it's not guaranteed.
    struct Line pxl_lines[4] = {
        (struct Line){.start = pxl->ul, .end = pxl->ur},
        (struct Line){.start = pxl->ur, .end = pxl->lr},
        (struct Line){.start = pxl->lr, .end = pxl->ll},
        (struct Line){.start = pxl->ll, .end = pxl->ul},
    };

    struct Line coord_lines[4] = {
        (struct Line){.start = coord, .end = pxl->ul},
        (struct Line){.start = coord, .end = pxl->ur},
        (struct Line){.start = coord, .end = pxl->ll},
        (struct Line){.start = coord, .end = pxl->lr},
    };

    for (unsigned int i = 0; i < 4; ++i) {
        for (unsigned int j = 0; j < 4; ++j) {
            struct IntersectResult res = lines_intersection(pxl_lines[i], coord_lines[j], eps);

            if (res.does_intersect && !res.intersect_is_endpoints) {
                return false;
            }
        }
    }

    return true;
}

bool
satfire_pixels_overlap(struct SFPixel const left[static 1], struct SFPixel const right[static 1],
                       double eps)
{
    // Check if they are equal first, then of course they overlap!
    if (satfire_pixels_approx_equal(left, right, eps)) {
        return true;
    }

    // Check the bounding boxes.
    if (!satfire_pixels_bounding_boxes_overlap(left, right, eps)) {
        return false;
    }

    // If pixels overlap, then at least 1 vertex from one pixel must be inside the boundary of
    // the other pixel or the pixels must have lines that intersect. In the case of one pixel
    // completely contained inside another (extremely unlikely), there would be no intersections
    // but all the points of one would be contained in another. In any other case, there must be
    // an intersection of lines.
    //
    // This is all by my own reasoning, not based on any math book or papers on geometry. I'm
    // assuming all pixels are convex quadrilaterals.

    // Check for intersecting lines between the pixels.
    struct Line left_pxl_lines[4] = {
        (struct Line){.start = left->ul, .end = left->ur},
        (struct Line){.start = left->ur, .end = left->lr},
        (struct Line){.start = left->lr, .end = left->ll},
        (struct Line){.start = left->ll, .end = left->ul},
    };

    struct Line right_pxl_lines[4] = {
        (struct Line){.start = right->ul, .end = right->ur},
        (struct Line){.start = right->ur, .end = right->lr},
        (struct Line){.start = right->lr, .end = right->ll},
        (struct Line){.start = right->ll, .end = right->ul},
    };

    for (unsigned i = 0; i < 4; ++i) {
        struct Line left = left_pxl_lines[i];

        for (unsigned j = 0; j < 4; ++j) {
            struct Line right = right_pxl_lines[j];

            struct IntersectResult res = lines_intersection(left, right, eps);

            if (res.does_intersect && !res.intersect_is_endpoints) {
                return true;
            }
        }
    }

    // Checking for intersecting lines didn't find anything. Now try seeing if one pixel is
    // contained in the other pixel.
    struct SFCoord left_coords[4] = {left->ul, left->ur, left->lr, left->ll};
    for (unsigned i = 0; i < 4; ++i) {
        if (satfire_pixel_contains_coord(right, left_coords[i], eps)) {
            return true;
        }
    }

    struct SFCoord right_coords[4] = {right->ul, right->ur, right->lr, right->ll};
    for (unsigned i = 0; i < 4; ++i) {
        if (satfire_pixel_contains_coord(left, right_coords[i], eps)) {
            return true;
        }
    }

    // No intersecting lines and no corners of one pixel contained in the other, so there
    // is no overlap.
    return false;
}

bool
satfire_pixels_are_adjacent_or_overlap(struct SFPixel const left[static 1],
                                       struct SFPixel const right[static 1], double eps)
{
    //
    // Try some shortcuts first
    //

    if (!satfire_pixels_bounding_boxes_overlap(left, right, eps)) {
        return false;
    }

    struct SFCoord left_coords[4] = {left->ul, left->ur, left->lr, left->ll};
    struct SFCoord right_coords[4] = {right->ul, right->ur, right->lr, right->ll};

    // Count the number of close coords
    unsigned int num_close_coords = 0;
    for (unsigned int i = 0; i < 4; ++i) {
        for (unsigned int j = 0; j < 4; ++j) {
            if (satfire_coord_are_close(left_coords[i], right_coords[j], eps)) {
                ++num_close_coords;

                // bail out early if we can
                if (num_close_coords > 1) {
                    return true;
                }
            }
        }
    }

    // Check if any points are contained in the other pixel
    for (unsigned int i = 0; i < 4; ++i) {
        if (satfire_pixel_contains_coord(right, left_coords[i], eps)) {
            return true;
        }

        if (satfire_pixel_contains_coord(left, right_coords[i], eps)) {
            return true;
        }
    }

    //
    // Fallback to the tested methods.
    //
    return satfire_pixels_overlap(left, right, eps) ||
           satfire_pixels_are_adjacent(left, right, eps);
}

bool
satfire_pixels_are_adjacent(struct SFPixel const left[static 1],
                            struct SFPixel const right[static 1], double eps)
{
    if (satfire_pixels_approx_equal(left, right, eps)) {
        return false;
    }

    if (!satfire_pixels_bounding_boxes_overlap(left, right, eps)) {
        return false;
    }

    struct SFCoord left_coords[4] = {left->ul, left->ur, left->lr, left->ll};
    struct SFCoord right_coords[4] = {right->ul, right->ur, right->lr, right->ll};

    // Count the number of close coords and mark which ones are close.
    bool left_close[4] = {false, false, false, false};
    bool right_close[4] = {false, false, false, false};
    unsigned int num_close_coords = 0;
    for (unsigned int i = 0; i < 4; ++i) {
        for (unsigned int j = 0; j < 4; ++j) {
            if (satfire_coord_are_close(left_coords[i], right_coords[j], eps)) {
                ++num_close_coords;
                left_close[i] = true;
                right_close[j] = true;
            }
        }
    }

    // bail out early if we can
    if (num_close_coords < 1 || num_close_coords > 2) {
        return false;
    }

    // Check if any not close points are contained in the other pixel
    for (unsigned int i = 0; i < 4; ++i) {
        if (!left_close[i]) {
            if (satfire_pixel_contains_coord(right, left_coords[i], eps)) {
                return false;
            }
        }

        if (!right_close[i]) {
            if (satfire_pixel_contains_coord(left, right_coords[i], eps)) {
                return false;
            }
        }
    }

    // The following is a heuristic that should catch most of the remaining edge cases. For the
    // satellite data this program will be working with, this should really be more than good
    // enough.

    // If they are adjacent, the centroid of neither should be interior to the other.
    struct SFCoord left_centroid = satfire_pixel_centroid(left);
    if (satfire_pixel_contains_coord(right, left_centroid, eps)) {
        return false;
    }
    struct SFCoord right_centroid = satfire_pixel_centroid(right);
    if (satfire_pixel_contains_coord(left, right_centroid, eps)) {
        return false;
    }

    return true;
}

/*-------------------------------------------------------------------------------------------------
 *                                         PixelList
 *-----------------------------------------------------------------------------------------------*/
static struct SFPixelList *
satfire_pixel_list_expand(struct SFPixelList plist[static 1])
{
    assert(plist);
    assert(plist->len <= plist->capacity);

    size_t new_capacity = (plist->capacity * 3) / 2;
    if (new_capacity <= plist->capacity) {
        new_capacity = 2 * plist->capacity;
    }

    plist = realloc(plist, sizeof(struct SFPixelList) + new_capacity * sizeof(struct SFPixel));
    Stopif(!plist, exit(EXIT_FAILURE), "unable to realloc, aborting");
    plist->capacity = new_capacity;

    return plist;
}

struct SFPixelList *
satfire_pixel_list_new()
{
    size_t const initial_capacity = 4;

    return satfire_pixel_list_new_with_capacity(initial_capacity);
}

struct SFPixelList *
satfire_pixel_list_new_with_capacity(size_t capacity)
{
    // We have to start at a minimal size of 2 for the 3/2 expansion factor to work (integer
    // aritmetic).
    if (capacity < 2) {
        capacity = 2;
    }

    struct SFPixelList *ptr =
        calloc(sizeof(struct SFPixelList) + capacity * sizeof(struct SFPixel), sizeof(char));

    Stopif(!ptr, exit(EXIT_FAILURE), "unable to calloc, aborting");

    ptr->capacity = capacity;

    assert(ptr->len == 0);
    return ptr;
}

struct SFPixelList *
satfire_pixel_list_destroy(struct SFPixelList plist[static 1])
{
    if (plist) {
        free(plist);
    }

    return 0;
}

struct SFPixelList *
satfire_pixel_list_copy(struct SFPixelList const *plist)
{
    assert(plist);
    assert(plist->len <= plist->capacity);

    size_t copy_size = plist->len >= 4 ? plist->len : 4;
    struct SFPixelList *copy = satfire_pixel_list_new_with_capacity(copy_size);
    memcpy(copy, plist, sizeof(struct SFPixelList) + plist->len * sizeof(struct SFPixel));

    return copy;
}

struct SFPixelList *
satfire_pixel_list_append(struct SFPixelList list[static 1], struct SFPixel const apix[static 1])
{
    assert(list);
    assert(list->len <= list->capacity);

    if (list->len == list->capacity) {
        list = satfire_pixel_list_expand(list);
    }

    list->pixels[list->len] = *apix;
    list->len++;

    assert(list);
    assert(list->len <= list->capacity);

    return list;
}

struct SFPixelList *
satfire_pixel_list_clear(struct SFPixelList list[static 1])
{
    assert(list);
    assert(list->len <= list->capacity);

    list->len = 0;
    return list;
}

struct SFCoord
satfire_pixel_list_centroid(struct SFPixelList const list[static 1])
{
    assert(list);
    assert(list->len <= list->capacity);

    struct SFCoord centroid = {.lat = 0.0, .lon = 0.0};
    for (unsigned int i = 0; i < list->len; ++i) {
        struct SFCoord coord = satfire_pixel_centroid(&list->pixels[i]);
        centroid.lat += coord.lat;
        centroid.lon += coord.lon;
    }

    centroid.lat /= (double)list->len;
    centroid.lon /= (double)list->len;

    return centroid;
}

double
satfire_pixel_list_total_power(struct SFPixelList const list[static 1])
{
    assert(list);
    assert(list->len <= list->capacity);

    double total_power = 0.0;

    for (unsigned int i = 0; i < list->len; ++i) {
        if (!isinf(list->pixels[i].power)) {
            total_power += list->pixels[i].power;
        }
    }

    return total_power;
}

double
satfire_pixel_list_total_area(struct SFPixelList const list[static 1])
{
    assert(list);
    assert(list->len <= list->capacity);

    double total_area = 0.0;

    for (unsigned int i = 0; i < list->len; ++i) {
        if (!isinf(list->pixels[i].area)) {
            total_area += list->pixels[i].area;
        }
    }

    return total_area;
}

double
satfire_pixel_list_max_temperature(struct SFPixelList const list[static 1])
{
    assert(list);
    assert(list->len <= list->capacity);

    double max_temperature = -HUGE_VAL;

    for (unsigned int i = 0; i < list->len; ++i) {
        max_temperature = fmax(list->pixels[i].temperature, max_temperature);
    }

    return max_temperature;
}

double
satfire_pixel_list_max_scan_angle(struct SFPixelList const list[static 1])
{
    assert(list);
    assert(list->len <= list->capacity);

    double max_scan_angle = -HUGE_VAL;

    for (unsigned int i = 0; i < list->len; ++i) {
        max_scan_angle = fmax(list->pixels[i].scan_angle, max_scan_angle);
    }

    return max_scan_angle;
}

bool
satfire_pixel_lists_adjacent_or_overlap(struct SFPixelList const left[static 1],
                                        struct SFPixelList const right[static 1], double eps)
{
    assert(left);
    assert(left->len <= left->capacity);

    assert(right);
    assert(right->len <= right->capacity);

    struct SFBoundingBox lb = satfire_pixel_list_bounding_box(left);
    struct SFBoundingBox rb = satfire_pixel_list_bounding_box(right);

    if (!satfire_bounding_boxes_overlap(&lb, &rb, eps)) {
        return false;
    }

    for (unsigned int l = 0; l < left->len; ++l) {
        struct SFPixel const *lp = &left->pixels[l];
        for (unsigned int r = 0; r < right->len; ++r) {
            struct SFPixel const *rp = &right->pixels[r];
            if (satfire_pixels_are_adjacent_or_overlap(lp, rp, eps)) {
                return true;
            }
        }
    }

    return false;
}

struct SFBoundingBox
satfire_pixel_list_bounding_box(struct SFPixelList const list[static 1])
{

    double min_lat = HUGE_VAL;
    double max_lat = -HUGE_VAL;
    double min_lon = HUGE_VAL;
    double max_lon = -HUGE_VAL;

    for (unsigned int l = 0; l < list->len; ++l) {
        struct SFPixel const *lp = &list->pixels[l];
        for (int i = 0; i < 4; ++i) {
            min_lat = fmin(min_lat, lp->coords[i].lat);
            min_lon = fmin(min_lon, lp->coords[i].lon);
            max_lat = fmax(max_lat, lp->coords[i].lat);
            max_lon = fmax(max_lon, lp->coords[i].lon);
        }
    }

    return (struct SFBoundingBox){.ll = (struct SFCoord){.lat = min_lat, .lon = min_lon},
                                  .ur = (struct SFCoord){.lat = max_lat, .lon = max_lon}};
}

/*-------------------------------------------------------------------------------------------------
 *                                         Binary Format
 *-----------------------------------------------------------------------------------------------*/
size_t
satfire_pixel_list_binary_serialize_buffer_size(struct SFPixelList const plist[static 1])
{
    return sizeof(struct SFPixelList) + sizeof(struct SFPixel) * plist->len;
}

size_t
satfire_pixel_list_binary_serialize(struct SFPixelList const plist[static 1], size_t buf_size,
                                    unsigned char buffer[buf_size])
{
    memcpy(buffer, plist, buf_size);

    return buf_size;
}

struct SFPixelList *
satfire_pixel_list_binary_deserialize(unsigned char const buffer[static sizeof(size_t)])
{
    // member len needs to be first for the current binary serialization scheme.
    size_t len = 0;
    memcpy(&len, buffer, sizeof(len));

    size_t buf_len = sizeof(struct SFPixelList) + sizeof(struct SFPixel) * len;

    struct SFPixelList *list = calloc(buf_len, sizeof(unsigned char));

    Stopif(!list, exit(EXIT_FAILURE), "out of memory, aborting");

    memcpy(list, buffer, buf_len);
    list->capacity = list->len;

    return list;
}

/*-------------------------------------------------------------------------------------------------
 *                                         KML Export
 *-----------------------------------------------------------------------------------------------*/
static void
satfire_pixel_list_kml_write_pixel_style(FILE *strm, double power)
{
    double const max_power = 3000.0;
    double const max_green_for_orange = 0.647;
    double const full_red_power = max_power / 2.0;

    double rd = 1.0;
    double gd = 0.0;
    double bd = 0.0;
    double ad = 0.6;

    if (isinf(power)) {
        power = max_power;
    }

    power = fmin(power, max_power);

    if (power <= full_red_power) {
        gd = (full_red_power - power) / full_red_power * max_green_for_orange;
    } else {
        gd = (power - full_red_power) / (max_power - full_red_power);
        bd = gd;
    }

    int ri = (int)(rd * 255);
    int gi = (int)(gd * 255);
    int bi = (int)(bd * 255);
    int ai = (int)(ad * 255);

    assert(ri < 256 && gi < 256 && bi < 256 && ai < 256);

    char color[9] = {0};
    sprintf(color, "%02x%02x%02x%02x", ai, bi, gi, ri);

    kamel_start_style(strm, 0);
    kamel_poly_style(strm, color, true, false);
    kamel_end_style(strm);

    return;
}

void
satfire_pixel_list_kml_write(FILE *strm, struct SFPixelList const plist[static 1])
{
    assert(plist);

    char desc[256] = {0};
    for (unsigned int i = 0; i < plist->len; ++i) {
        struct SFPixel pixel = plist->pixels[i];

        int num_printed = sprintf(desc,
                                  "Power: %.0lfMW<br/>"
                                  "Area: %.0lf m^2</br>"
                                  "Temperature: %.0lf&deg;K<br/>"
                                  "scan angle: %.0lf&deg;<br/>"
                                  "Mask Flag: %s<br/>"
                                  "Data Quality Flag: %s<br/>",
                                  pixel.power, pixel.area, pixel.temperature, pixel.scan_angle,
                                  satfire_satellite_mask_code_to_string(pixel.mask_flag),
                                  satfire_satellite_dqf_code_to_string(pixel.data_quality_flag));

        Stopif(num_printed >= sizeof(desc), exit(EXIT_FAILURE), "description buffer too small");

        kamel_start_placemark(strm, 0, desc, 0);

        satfire_pixel_list_kml_write_pixel_style(strm, pixel.power);

        kamel_start_polygon(strm, true, true, "clampToGround");
        kamel_polygon_start_outer_ring(strm);
        kamel_start_linear_ring(strm);

        for (unsigned int j = 0; j < sizeof(pixel.coords) / sizeof(pixel.coords[0]); ++j) {
            struct SFCoord coord = pixel.coords[j];
            kamel_linear_ring_add_vertex(strm, coord.lat, coord.lon, 0.0);
        }
        // Close the loop.
        struct SFCoord coord = pixel.coords[0];
        kamel_linear_ring_add_vertex(strm, coord.lat, coord.lon, 0.0);

        kamel_end_linear_ring(strm);
        kamel_polygon_end_outer_ring(strm);
        kamel_end_polygon(strm);

        kamel_end_placemark(strm);
    }

    return;
}
*/
