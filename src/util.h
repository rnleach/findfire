#pragma once

#include <assert.h>
#include <math.h>
//#include <ctype.h>
//#include <stdbool.h>
//#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
static inline char const *
file_ext(const char *fname)
{
    const char *dot = strrchr(fname, '.');
    if (!dot || dot == fname)
        return "";
    return dot + 1;
}

static inline char const *
get_file_name(char const *full_path)
{
    const char *slash = strrchr(full_path,'/');
    if (!slash) {
        return full_path;
    }

    return slash + 1;
}


