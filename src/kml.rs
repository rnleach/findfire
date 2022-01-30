pub(crate) struct KmlFile {}

/*

/** \file kamel.h
 * \brief Small library for producing KML files.
 *
 * This library is extremely incomplete as it only implements the parts of KML I need to use in my
 * projects. To use this library, copy the kamel.h and kamel.c files into the source directory of
 * your project.
 */

#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <time.h>

/*-------------------------------------------------------------------------------------------------
 *                            Documents, Folders, Organization.
 *-----------------------------------------------------------------------------------------------*/
/** \brief Write out a KML header for a file opening the Document element. */
void kamel_start_document(FILE *output);

/** \brief Write out a KML footer for a file closing the Document element. */
void kamel_end_document(FILE *output);

/** \brief Start a KML folder. */
void kamel_start_folder(FILE *output, char const *name, char const *description, bool is_open);

/** \brief Close a folder element. */
void kamel_end_folder(FILE *output);

/** \brief Start a Placemark. */
void kamel_start_placemark(FILE *output, char const *name, char const *description,
                           char const *style_url);

/** \brief Finish a Placemark. */
void kamel_end_placemark(FILE *output);

/*-------------------------------------------------------------------------------------------------
 *                                          Style
 *-----------------------------------------------------------------------------------------------*/
/** Start a style definition. */
void kamel_start_style(FILE *output, char const *style_id);

/** End a style definition. */
void kamel_end_style(FILE *output);

/** Create a PolyStyle element.
 *
 * This should only go inside a style element.
 */
void kamel_poly_style(FILE *output, char const *color, bool filled, bool outlined);

/** Create a IconStyle element.
 *
 * This should only go inside a style element.
 */
void kamel_icon_style(FILE *output, char const *icon_url, double scale);

/*-------------------------------------------------------------------------------------------------
 *                                          Time
 *-----------------------------------------------------------------------------------------------*/
/** Create a TimeSpan element. */
void kamel_timespan(FILE *output, time_t start, time_t end);

/*-------------------------------------------------------------------------------------------------
 *                                       Geometry
 *-----------------------------------------------------------------------------------------------*/
/** Start a MultiGeometry. */
void kamel_start_multigeometry(FILE *output);

/** End a MultiGeometry. */
void kamel_end_multigeometry(FILE *output);

/** Start a Polygon. */
void kamel_start_polygon(FILE *output, bool extrude, bool tessellate, char const *altitudeMode);

/** End a Polygon. */
void kamel_end_polygon(FILE *output);

/** Start the polygon outer ring.
 *
 * This should only be used inside a Polygon element.
 */
void kamel_polygon_start_outer_ring(FILE *output);

/** End the polygon outer ring.
 *
 * This should only be used inside a Polygon element.
 */
void kamel_polygon_end_outer_ring(FILE *output);

/** Start a LinearRing. */
void kamel_start_linear_ring(FILE *output);

/** End a LinearRing. */
void kamel_end_linear_ring(FILE *output);

/** Add a vertex to the LinearRing.  */
void kamel_linear_ring_add_vertex(FILE *output, double lat, double lon, double z);

/** Write out a KML Point. */
void kamel_point(FILE *output, double lat, double lon, double z);
#include "kamel.h"

#include <string.h>

/*
 * Copyright (c) 2021 Ryan Leach
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 */

/*-------------------------------------------------------------------------------------------------
 *                                  Utility Routines
 *-----------------------------------------------------------------------------------------------*/
static void
kamel_write_description(FILE *output, char const *description)
{
    assert(description);
    fprintf(output, "<description><![CDATA[%s]]></description>\n", description);
}

/*-------------------------------------------------------------------------------------------------
 *                            Documents, Folders, Organization.
 *-----------------------------------------------------------------------------------------------*/
void
kamel_start_document(FILE *output)
{
    assert(output);
    static char const *header = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                                "<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n"
                                "<Document>\n";
    fputs(header, output);
    return;
}

void
kamel_end_document(FILE *output)
{
    assert(output);
    static char const *footer = "</Document>\n</kml>\n";
    fputs(footer, output);
    return;
}

void
kamel_start_folder(FILE *output, char const *name, char const *description, bool is_open)
{
    assert(output);

    fputs("<Folder>\n", output);

    if (name) {
        fprintf(output, "<name>%s</name>\n", name);
    }

    if (description) {
        kamel_write_description(output, description);
    }

    if (is_open) {
        fputs("<open>1</open>\n", output);
    }

    return;
}

void
kamel_end_folder(FILE *output)
{
    assert(output);

    fputs("</Folder>\n", output);

    return;
}

void
kamel_start_placemark(FILE *output, char const *name, char const *description,
                      char const *style_url)
{
    assert(output);

    fprintf(output, "<Placemark>\n");

    if (name) {
        fprintf(output, "<name>%s</name>\n", name);
    }

    if (description) {
        kamel_write_description(output, description);
    }

    if (style_url) {
        fprintf(output, "<styleUrl>%s</styleUrl>\n", style_url);
    }

    return;
}

void
kamel_end_placemark(FILE *output)
{
    assert(output);
    fputs("</Placemark>\n", output);
    return;
}

/*-------------------------------------------------------------------------------------------------
 *                                          Style
 *-----------------------------------------------------------------------------------------------*/
void
kamel_start_style(FILE *output, char const *style_id)
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
kamel_end_style(FILE *output)
{
    assert(output);
    fputs("</Style>\n", output);
    return;
}

void
kamel_poly_style(FILE *output, char const *color, bool filled, bool outlined)
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
kamel_icon_style(FILE *output, char const *icon_url, double scale)
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

/*-------------------------------------------------------------------------------------------------
 *                                          Time
 *-----------------------------------------------------------------------------------------------*/
void
kamel_timespan(FILE *output, time_t start, time_t end)
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

/*-------------------------------------------------------------------------------------------------
 *                                       Geometry
 *-----------------------------------------------------------------------------------------------*/
void
kamel_start_multigeometry(FILE *output)
{
    assert(output);
    fputs("<MultiGeometry>\n", output);
    return;
}

void
kamel_end_multigeometry(FILE *output)
{
    assert(output);
    fputs("</MultiGeometry>\n", output);
    return;
}

void
kamel_start_polygon(FILE *output, bool extrude, bool tessellate, char const *altitudeMode)
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
kamel_end_polygon(FILE *output)
{
    assert(output);
    fputs("</Polygon>\n", output);
    return;
}

void
kamel_polygon_start_outer_ring(FILE *output)
{
    assert(output);
    fputs("<outerBoundaryIs>\n", output);
    return;
}

void
kamel_polygon_end_outer_ring(FILE *output)
{
    assert(output);
    fputs("</outerBoundaryIs>\n", output);
    return;
}

void
kamel_start_linear_ring(FILE *output)
{
    assert(output);
    fputs("<LinearRing>\n", output);
    fputs("<coordinates>\n", output);
    return;
}

void
kamel_end_linear_ring(FILE *output)
{
    assert(output);
    fputs("</coordinates>\n", output);
    fputs("</LinearRing>\n", output);
    return;
}

void
kamel_linear_ring_add_vertex(FILE *output, double lat, double lon, double z)
{
    assert(output);
    fprintf(output, "%lf,%lf,%lf\n", lon, lat, z);
    return;
}

void
kamel_point(FILE *output, double lat, double lon, double z)
{
    assert(output);
    fprintf(output, "<Point>\n<coordinates>%lf,%lf,%lf</coordinates>\n</Point>\n", lon, lat, z);
    return;
}
*/
