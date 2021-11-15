#pragma once
/** \file util.h
 * \brief Utility functions and macros used throughout the project.
 *
 * The things in this module don't fit nicely in other modules.
 */

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <time.h>

#include <dirent.h>

/*-------------------------------------------------------------------------------------------------
 *                                        Error handling.
 *-----------------------------------------------------------------------------------------------*/
/** \brief Clean error handling not removed in release builds.
 *
 * Unlike \c assert, this macro isn't compiled out if \c NDEBUG is defined. Error checking that is
 * always on.
 */
#define Stopif(assertion, error_action, ...)                                                       \
    {                                                                                              \
        if (assertion) {                                                                           \
            fprintf(stderr, __VA_ARGS__);                                                          \
            fprintf(stderr, "\n");                                                                 \
            {                                                                                      \
                error_action;                                                                      \
            }                                                                                      \
        }                                                                                          \
    }

/*-------------------------------------------------------------------------------------------------
 *                                     File name handling.
 *-----------------------------------------------------------------------------------------------*/
/** \brief Find the file extension.
 *
 * Finds the part of the path after the last '.' in the file name (\a fname).
 *
 * \param fname should be a file name without any slashes, '/'. That is, it's just the file name
 * not the containing directories. If it does include the containing directories, it still may
 * work, but the error conditions discussed below may not.
 *
 * \returns A pointer into the original \a fname to the start of the extension. If the file name
 * doesn't contain a '.' character, then it returns a pointer to an empty string, "".
 */
char const *file_ext(const char *fname);

/** \brief Find the file name in a path.
 *
 * \returns a pointer to the first character after the last '/' character. This is not guaranteed
 * to be a file name, it could be a directory if the path didn't include a file name. If there is
 * no '/' character in the path, then it returns the whole path.
 */
char const *get_file_name(char const *full_path);

/*-------------------------------------------------------------------------------------------------
 *                                     Time parsing.
 *-----------------------------------------------------------------------------------------------*/
/** \brief Parse a date-time from a substring from a file name.
 *
 * The GOES data stored via the NOAA Big Data initiative is stored in files that include the
 * scan start and end times in the file names. The format of that time stamp is YYYYJJJHHMMSS,
 * where:
 *     YYYY is the year
 *     JJJ is the day of the year (1-366)
 *     HH is the hour of the day (0-23)
 *     MM is the minute of the hour (0-59)
 *     SS is the seconds of the minute (0-59)
 *
 * \param tstr is a pointer to the first character of the time stamp, that is the first 'Y' in the
 * YYYYJJJHHMMSS format described above.
 *
 * \returns a unix timestamp.
 */
time_t parse_time_string(char const *tstr);

/*-------------------------------------------------------------------------------------------------
 *                                     Walk a Directory Tree
 *-----------------------------------------------------------------------------------------------*/
/* The maximum depth of the directory tree. */
#define DIR_STACK_DEPTH 10
#define MAX_PATH_LEN 256

/** \brief A stack to keep track of the state while we walk a directory tree.
 *
 * Order doesn't matter (inorder, preorder, postorder, level order). This just visits every file
 * in a directory tree starting with the given root.
 */
struct DirWalkState {
    DIR *stack[DIR_STACK_DEPTH];
    char *paths[DIR_STACK_DEPTH];
    char current_entry_path[MAX_PATH_LEN];
    unsigned int top;
};

#undef DIR_STACK_DEPTH
#undef MAX_PATH_LEN

/** Create a new DirWalkState rooted at the given path. */
struct DirWalkState dir_walk_new_with_root(char const *root);

/** Cleanup and free any allocated memory associated with the DirWalkState. */
void dir_walk_destroy(struct DirWalkState done[static 1]);

/** Get the next regular file entry.
 *
 * Directiories are not returned, only regular files. If a directory is encountered it will be
 * added to the stack and all files under that root will be returned.
 */
char const *dir_walk_next_path(struct DirWalkState stck[static 1]);
