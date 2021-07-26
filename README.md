# findfire
Analysis of GOES-R/S NetCDF4 Fire Detection Characteristics files.

(Goal - we're not there yet)

Given a directory containing *-FDCC-*, *-FDCF-*, or *-FDCM-* files (Fire Detection Characteristics)
from GOES-R (GOES-16) and GOES-S (GOES-17) satellites, this program will analyze all of them in 
chronological order. The analysis finds clusters of pixels that are connected and analyzes their 
mean latitude, mean longitude, and total fire power in megawatts. Then the points in the time series
are connected to track individual fires.

This initial version will treat fires from each satellite (GOES-16 and GOES-17) independently as
well as fire from each scanning sector (CONUS [FDCC], Full Disk [FDCF], and Mesosector [FDCM])
independenly. Later versions may try to combine the time series from different satellites and
sectors together.

This initial version will also rely on the file naming convention used by the NOAA Big Data
initiative to detect satellite, sector, scan start, and scan end times. Later versions may use
attributes in the NetCDF4 to detect these properties internally.


## Dependencies

### GLIB
I'm developing this on Linux and my other computer is a Mac. So I know this will work, however, if
portability to windows becomes an issue, it may be easier to just write my own data structures.

So far I'm only using the following from GLIB:
 - GArray


### GDAL (3.2.2 or later used in development)
 This is critical for accessing and geo-referencing the data. Whatever version of GDAL you're using,
 it must have support for NetCDF4 installed as well. This shouldn't be a problem since that is the
 default anyway.


### SQLITE3
 sqlite3 is used to keep track of the detected fires so they can be connected and tracked throughout
 time. (Planned).
