# satfire
Analysis of GOES-R/S NetCDF4 Fire Detection Characteristics files.

(Goal - we're not there yet)

Given a directory containing *-FDCC-*, *-FDCF-*, or *-FDCM-* files (Fire Detection Characteristics)
from GOES-R (GOES-16) and GOES-S (GOES-17) satellites, this program will analyze all of them. 
The analysis finds clusters of pixels that are connected and analyzes their mean latitude, mean
longitude, and total fire power in megawatts.

We rely on the file naming convention used by the NOAA Big Data initiative to detect satellite, 
sector, scan start, and scan end times. 

Currently all directories and "options" are hard coded in the executable files. In the future there
may be configuration files and/or command line arguments.

## Programs

### findfire
 Find fire analyzes the netCDF files for clusters and stores the data about the clusters in an
 intermediary database. This program looks at 1 satellite file at a time.

### connectfire
 Takes the intermediary database created by findfire and connects the clusters in time, and stores 
 the results in another database. Each cluster in the final database will have an index code 
 assigned to it. That index code will relate to another table with summary information about that
 fire.

### export_kml
 Exports the results of connect fire as a kml file for viewing. Currently only exports the final
 fire perimeters, but there's a lot that could be done here.
