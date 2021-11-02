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

    // TODO: Implement
    assert(false);

    return;
}

void
kml_end_document(FILE *output)
{
    assert(output);

    // TODO: Implement
    assert(false);

    return;
}

void
kml_start_placemark(FILE *output, char const *name, char const *description, char const *style_url)
{
    assert(output);

    // TODO: Implement
    assert(false);

    return;
}

void
kml_end_placemark(FILE *output)
{
    assert(output);

    // TODO: Implement
    assert(false);

    return;
}

void
kml_start_style(FILE *output, char const *style_id)
{
    assert(output);

    // TODO: Implement
    assert(false);

    return;
}

void
kml_end_style(FILE *output)
{
    assert(output);

    // TODO: Implement
    assert(false);

    return;
}

void
kml_poly_style(FILE *output, char const *color, bool filled, bool outlined)
{
    assert(output);

    // TODO: Implement
    assert(false);

    return;
}

void
kml_timespan(FILE *output, time_t start, time_t end)
{
    assert(output);

    // TODO: Implement
    assert(false);

    return;
}
