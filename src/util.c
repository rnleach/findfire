#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#include "util.h"

/*-------------------------------------------------------------------------------------------------
 *                                     File name handling.
 *-----------------------------------------------------------------------------------------------*/
char const *
file_ext(const char *fname)
{
    const char *dot = strrchr(fname, '.');
    if (!dot || dot == fname)
        return "";
    return dot + 1;
}

char const *
get_file_name(char const *full_path)
{
    const char *slash = strrchr(full_path, '/');
    if (!slash) {
        return full_path;
    }

    return slash + 1;
}

/*-------------------------------------------------------------------------------------------------
 *                                     Time parsing.
 *-----------------------------------------------------------------------------------------------*/
// TODO: make thread safe
//   check memcpy, memset, atoi, mktime
time_t
parse_time_string(char const *tstr)
{
    char buff[5] = {0};

    memcpy(buff, tstr, 4);
    int year = atoi(buff);
    bool leap_year = year % 4 == 0;
    if (leap_year && year % 100 == 0) {
        leap_year = false;
    }
    if (!leap_year && year % 400 == 0) {
        leap_year = true;
    }
    year -= 1900;

    memset(buff, 0, sizeof(buff));
    memcpy(buff, tstr + 4, 3);
    int doy = atoi(buff);
    int month = 0;
    int day = 0;
    for (int i = 1; i < 12; i++) {
        int days_in_month = 31;
        if (i == 2) {
            if (leap_year) {
                days_in_month = 29;
            } else {
                days_in_month = 28;
            }
        }
        if (i == 4 || i == 6 || i == 9 || i == 11) {
            days_in_month = 30;
        }

        if (doy > days_in_month) {
            month = i;
            doy -= days_in_month;
        } else {
            day = doy;
            break;
        }
    }

    memset(buff, 0, sizeof(buff));
    memcpy(buff, tstr + 7, 2);
    int hour = atoi(buff);

    memset(buff, 0, sizeof(buff));
    memcpy(buff, tstr + 9, 2);
    int min = atoi(buff);

    memset(buff, 0, sizeof(buff));
    memcpy(buff, tstr + 11, 2);
    int sec = atoi(buff);

    struct tm parsed_time = {0};

    parsed_time.tm_year = year;
    parsed_time.tm_mon = month;
    parsed_time.tm_mday = day;
    parsed_time.tm_hour = hour;
    parsed_time.tm_min = min;
    parsed_time.tm_sec = sec;

    assert(month < 12);
    assert(day <= 31);
    assert(hour < 24);
    assert(min < 60);
    assert(sec < 60);

    return mktime(&parsed_time);
}

/*-------------------------------------------------------------------------------------------------
 *                                     Walk a Directory Tree
 *-----------------------------------------------------------------------------------------------*/

struct DirWalkState
dir_walk_new_with_root(char const *root)
{
    DIR *dir = opendir(root);
    Stopif(!dir, exit(EXIT_FAILURE), "Error opening root directory: %s", root);

    char *root_copy = 0;
    int num_chars = asprintf(&root_copy, "%s", root);
    Stopif(num_chars <= 0, exit(EXIT_FAILURE), "Unable to copy root path: %s", root);

    return (struct DirWalkState){
        .stack = {[0] = dir}, .paths = {[0] = root_copy}, .top = 0, .current_entry_path = {0}};
}

void
dir_walk_destroy(struct DirWalkState done[static 1])
{
    for (unsigned int i = 0; i <= done->top; i++) {
        closedir(done->stack[i]);
        free(done->paths[i]);
        done->paths[i] = 0;
    }

    done->top = 0;
}

static void
dir_walk_state_pop(struct DirWalkState state[static 1])
{
    assert(state->top > 0);

    if (state->stack[state->top]) {
        closedir(state->stack[state->top]);
    }

    if (state->paths[state->top]) {
        free(state->paths[state->top]);
    }

    state->paths[state->top] = 0;
    state->top--;

    return;
}

static void
dir_walk_state_push(struct DirWalkState state[static 1], struct dirent entry[static 1])
{
    assert(entry->d_type == DT_DIR);

    state->top++;

    Stopif(state->top >= sizeof(state->stack), state->top--, "Stack too deep, skipping: %s/%s",
           state->paths[state->top - 1], entry->d_name);

    int chars_printed =
        asprintf(&state->paths[state->top], "%s/%s", state->paths[state->top - 1], entry->d_name);
    Stopif(chars_printed < 0, exit(EXIT_FAILURE), "Error saving path to next level.");

    state->stack[state->top] = opendir(state->paths[state->top]);

    Stopif(!state->stack[state->top], dir_walk_state_pop(state), "Error opening directory: %s/%s",
           state->paths[state->top], entry->d_name);

    return;
}

static bool
dir_walk_state_set_current_entry_path(struct DirWalkState state[static 1],
                                      struct dirent entry[static 1])
{
    int chars_printed = snprintf(state->current_entry_path, sizeof(state->current_entry_path),
                                 "%s/%s", state->paths[state->top], entry->d_name);

    Stopif(chars_printed >= sizeof(state->current_entry_path), return false,
           "File name buffer too small, skipping: %s/%s", state->paths[state->top], entry->d_name);

    return true;
}

char const *
dir_walk_next_path(struct DirWalkState state[static 1])
{
    struct dirent *entry = 0;
    do {
        entry = readdir(state->stack[state->top]);

        while (!entry && state->top != 0) {
            dir_walk_state_pop(state);
            entry = readdir(state->stack[state->top]);
        }

        if (entry && entry->d_type == DT_DIR && entry->d_name[0] != '.') {
            dir_walk_state_push(state, entry);
        } else if (entry && entry->d_type == DT_REG) {

            if (dir_walk_state_set_current_entry_path(state, entry)) {
                return &state->current_entry_path[0];
            }
        }
    } while (entry);

    // If we get here, we've run out of entries.
    return 0;
}

/*-------------------------------------------------------------------------------------------------
 *                                     Create KML Files
 *-----------------------------------------------------------------------------------------------*/
void
kml_start_document(FILE *output)
{
    assert(output);
    static char const *header = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                                "<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n"
                                "<Document>\n";
    fputs(header, output);
    return;
}

void
kml_end_document(FILE *output)
{
    assert(output);
    static char const *footer = "</Document>\n</kml>\n";
    fputs(footer, output);
    return;
}

void
kml_start_folder(FILE *output, char const *name, char const *description, bool is_open)
{
    assert(output);

    fputs("<Folder>\n", output);

    if (name) {
        fprintf(output, "<name>%s</name>\n", name);
    }

    if (description) {
        fprintf(output, "<description>%s</description>\n", description);
    }

    if (is_open) {
        fputs("<open>1</open>\n", output);
    }

    return;
}

void
kml_end_folder(FILE *output)
{
    assert(output);

    fputs("</Folder>\n", output);

    return;
}

void
kml_start_placemark(FILE *output, char const *name, char const *description, char const *style_url)
{
    assert(output);

    fprintf(output, "<Placemark>\n");

    if (name) {
        fprintf(output, "<name>%s</name>\n", name);
    }

    if (description) {
        fprintf(output, "<description><![CDATA[%s]]></description>\n", description);
    }

    if (style_url) {
        fprintf(output, "<styleUrl>%s</styleUrl>\n", style_url);
    }

    return;
}

void
kml_end_placemark(FILE *output)
{
    assert(output);
    fputs("</Placemark>\n", output);
    return;
}

void
kml_start_style(FILE *output, char const *style_id)
{
    assert(output);

    if (style_id) {
        fprintf(output, "<Style id=\"%s\">\n", style_id);
    } else {
        fputs("<Style>\n", output);
    }
    return;
}

void
kml_end_style(FILE *output)
{
    assert(output);
    fputs("</Style>\n", output);
    return;
}

void
kml_poly_style(FILE *output, char const *color, bool filled, bool outlined)
{
    assert(output);
    fputs("<PolyStyle>\n", output);

    if (color) {
        fprintf(output, "<color>%s</color>\n", color);
        fputs("<colorMode>normal</colorMode>\n", output);
    } else {
        fputs("<colorMode>random</colorMode>\n", output);
    }

    fprintf(output, "<fill>%d</fill>\n", filled ? 1 : 0);
    fprintf(output, "<outline>%d</outline>\n", outlined ? 1 : 0);

    fputs("</PolyStyle>\n", output);

    return;
}

void
kml_icon_style(FILE *output, char const *icon_url, double scale)
{
    assert(output);

    fputs("<IconStyle>\n", output);

    if (scale > 0.0) {
        fprintf(output, "<scale>%lf</scale>\n", scale);
    } else {
        fputs("<scale>1</scale>\n", output);
    }

    if (icon_url) {
        fprintf(output, "<Icon><href>%s</href></Icon>\n", icon_url);
    }

    fputs("</IconStyle>\n", output);
    return;
}

void
kml_timespan(FILE *output, time_t start, time_t end)
{
    assert(output);
    struct tm start_tm = {0};
    struct tm end_tm = {0};

    gmtime_r(&start, &start_tm);
    gmtime_r(&end, &end_tm);

    char start_str[25] = {0};
    char end_str[25] = {0};

    strftime(start_str, sizeof(start_str), "%Y-%m-%dT%H:%M:%S.000Z", &start_tm);
    strftime(end_str, sizeof(end_str), "%Y-%m-%dT%H:%M:%S.000Z", &end_tm);

    fputs("<TimeSpan>\n", output);
    fprintf(output, "<begin>%s</begin>\n", start_str);
    fprintf(output, "<end>%s</end>\n", end_str);
    fputs("</TimeSpan>\n", output);

    return;
}

void
kml_start_multigeometry(FILE *output)
{
    assert(output);
    fputs("<MultiGeometry>\n", output);
    return;
}

void
kml_end_multigeometry(FILE *output)
{
    assert(output);
    fputs("</MultiGeometry>\n", output);
    return;
}

void
kml_start_polygon(FILE *output, bool extrude, bool tessellate, char const *altitudeMode)
{
    assert(output);

    fputs("<Polygon>\n", output);

    if (altitudeMode) {
        assert(strcmp(altitudeMode, "clampToGround") == 0 ||
               strcmp(altitudeMode, "relativeToGround") == 0 ||
               strcmp(altitudeMode, "absolute") == 0);

        fprintf(output, "<altitudeMode>%s</altitudeMode>\n", altitudeMode);
    }

    if (extrude) {
        fputs("<extrude>1</extrude>\n", output);
    }

    if (tessellate) {
        fputs("<tessellate>1</tessellate>\n", output);
    }

    return;
}

void
kml_end_polygon(FILE *output)
{
    assert(output);
    fputs("</Polygon>\n", output);
    return;
}

void
kml_polygon_start_outer_ring(FILE *output)
{
    assert(output);
    fputs("<outerBoundaryIs>\n", output);
    return;
}

void
kml_polygon_end_outer_ring(FILE *output)
{
    assert(output);
    fputs("</outerBoundaryIs>\n", output);
    return;
}

void
kml_start_linear_ring(FILE *output)
{
    assert(output);
    fputs("<LinearRing>\n", output);
    fputs("<coordinates>\n", output);
    return;
}

void
kml_end_linear_ring(FILE *output)
{
    assert(output);
    fputs("</coordinates>\n", output);
    fputs("</LinearRing>\n", output);
    return;
}

void
kml_linear_ring_add_vertex(FILE *output, double lat, double lon, double z)
{
    assert(output);
    fprintf(output, "%lf,%lf,%lf\n", lon, lat, z);
    return;
}

void
kml_point(FILE *output, double lat, double lon)
{
    assert(output);
    fprintf(output, "<Point>\n<coordinates>%lf,%lf,0.0</coordinates>\n</Point>\n", lon, lat);
    return;
}
