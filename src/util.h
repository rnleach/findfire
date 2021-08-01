#pragma once

#include <assert.h>
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
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

/*-------------------------------------------------------------------------------------------------
 *                                     Time parsing.
 *-----------------------------------------------------------------------------------------------*/
static inline time_t
parse_time_string(char const *tstr)
{
    char buff[5] = {0};

    memcpy(buff, tstr, 4);
    int year = atoi(buff);
    memset(buff, 0, sizeof(buff));
    bool leap_year = year % 4 == 0;
    if(leap_year && year % 100 == 0) {
        leap_year = false;
    }
    if(!leap_year && year % 400 == 0) {
        leap_year = true;
    }
    year -= 1900;

    memcpy(buff, tstr + 4, 3);
    int doy = atoi(buff);
    memset(buff, 0, sizeof(buff));
    int month = 0;
    int day = 0;
    for(int i = 1; i < 12; i++) {
        int days_in_month = 31;
        if(i == 2) {
            if(leap_year) {
                days_in_month = 29;
            } else {
                days_in_month = 28;
            }
        }
        if(i==4 || i==6 || i == 9 || i == 11) {
            days_in_month = 30;
        }

        if(doy > days_in_month) {
            month = i;
            doy -= days_in_month;
        } else {
            day = doy;
        }
    }

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

    return mktime(&parsed_time);
}
