#pragma once

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

/*
 * Version 1.0.0 - November 15th, 2021
 */

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

