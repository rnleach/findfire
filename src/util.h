#pragma once

#include <assert.h>
#include <stdio.h>
#include <time.h>

/*-------------------------------------------------------------------------------------------------
 *                                        Error handling.
 *-----------------------------------------------------------------------------------------------*/
/** Clean error handling. */
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
/** Find the file extension.
 *
 * Finds the part of the path after the last '.' in the file name (fname).
 *
 * \returns A pointer into the original fname to the start of the extension. If the file name 
 * doesn't contain a '.' character, then it returns a pointer to an empty string, "".
 */
char const * file_ext(const char *fname);

/** Find the file name in a path.
 *
 * \returns a pointer to the first character after the last '/' character. This is not guaranteed
 * to be a file name, it could be a directory if the path didn't include a file name. If there is 
 * no '/' character in the path, then it returns the whole path.
 */
char const * get_file_name(char const *full_path);

/*-------------------------------------------------------------------------------------------------
 *                                     Time parsing.
 *-----------------------------------------------------------------------------------------------*/
/** Parse a date-time from a substring from a file name.
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

