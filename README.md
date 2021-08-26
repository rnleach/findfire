# satfire
Analysis of GOES-R/S NetCDF4 Fire Detection Characteristics files.

(Goal - we're not there yet)

Given a directory containing *-FDCC-*, *-FDCF-*, or *-FDCM-* files (Fire Detection Characteristics)
from GOES-R (GOES-16) and GOES-S (GOES-17) satellites, this program will analyze all of them. 
The analysis finds clusters of pixels that are connected and analyzes their mean latitude, mean
longitude, and total fire power in megawatts.

This initial version will rely on the file naming convention used by the NOAA Big Data initiative 
to detect satellite, sector, scan start, and scan end times. Later versions may use attributes in
the NetCDF4 to detect these properties internally.

## Programs

### findfire
 Find fire analyzes the netCDF files for clusters and stores the data about the clusters in an
 intermediary database. This program looks at 1 satellite file at a time.

### connectfire
 Takes the intermediary database created by findfire and connects the clusters in time, and stores 
 the results in another database. Each cluster in the final database will have an index code 
 assigned to it. Codes with the same (6-character) prefix represent the same initial fire. 
 If at a time step there are multiple clusters that should be assigned to the same fire from a
 previous time step, more characters will be appended to the code. So as a fire grows, it may split
 into several smaller fires, and all fires with the same prefix came from the same original fire.

 If a cluster could be assigned to multiple different clusters from a previous time step, then it 
 will be assigned to the larger cluster. This is how mergers of fires are currently handled.

## Dependencies

### C Libraries

#### GDAL (3.2.2 or later used in development)
 This is critical for accessing and geo-referencing the data. Whatever version of GDAL you're using,
 it must have support for NetCDF4 installed as well. This shouldn't be a problem since that is the
 default.


#### SQLITE3
 sqlite3 is used to keep track of the detected fires so they can be connected and tracked throughout
 time.

### Rust crates available on crates.io

#### chrono
 Date-time types are critical for the whole project.

#### crossbeam-channel
 For concurrency and the ability to process multiple files at a time.

#### gdal and gdal-sys
 The Rust interface to GDAL.

#### rusqlite
 The rust interface to SQLITE3

#### walkdir
 For listing all the files in the data directory. Currently this may not be necessary, but in the 
 future I may go to a more structured directory tree for organizing the data instead of putting all
 the files in the same directory. Then this will be useful for walking the directory tree.
