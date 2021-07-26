#pragma once

#include <assert.h>
#include <math.h>
//#include <ctype.h>
//#include <stdbool.h>
//#include <stdio.h>
#include <stdlib.h>
//#include <string.h>


#define DEG2RAD (M_PI / 360.0 * 2.0)

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

