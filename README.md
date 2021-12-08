# SatFire
A collection of programs for working with GOES-16/17 Fire Detection Characterstics files to analyze
wildfire detections.

## findfire
Analysis of GOES-R/S NetCDF4 Fire Detection Characteristics files.

Given a directory containing *-FDCC-*, *-FDCF-*, or *-FDCM-* files (Fire Detection Characteristics)
from GOES-16/17 satellites, this program will analyze them. The analysis finds clusters of pixels
that are connected. The clusters are then stored in a database with the name of the satellite, the
scan sector, the scan angle (see below), cluster centroid latitude and longitude, the total fire 
power of the cluster in megawatts, and a binary representation of the image pixels that make up the 
cluster. 

The scan angle is the distance of the centroid of a pixel or cluster from the respective satellites
nadir position on the earth. The distance is in degrees latitude and longitude, so it represents the
angle between the line from the center of the Earth to nadir and the line from the center of the
Earth to the centroid. This is a good proxy for measuring how close a pixel or cluster is to the
limb of the Earth as viewed by the satellite, which is useful for some quality control.

The binary representation stored in the database includes the 4 corner coordinates, the scan angle 
and the fire power in megawatts of each pixel in the cluster. This basically represents all of the
original data that was used to construct the cluster.

The findfire program relies on the file naming convention used by the NOAA Big Data initiative to
detect satellite name, sector, scan start, and scan end times. Later versions may use attributes in
the NetCDF4 to detect these properties internally.

## showfire
Select clusters from the database and output them in a KML format.

This is a command line application that will select clusters based on a given start time, end time,
and geographic bounding box and then output them in KML. The KML elements include a time stamp for
the scan start and end times so the KML can be animated in Google Earth.

## currentfire
Select the clusters from the most recent satellite image given a satellite name and sector name.

This command line application will query the database for the most recent image given the satelltie
and sector and produce a KML file with all the clusters.

## connectfire
Create a database with the necessary information to create time series of fires. (Not Implemented)

This program will scan the cluster database and connect clusters from different scan times together
based on their geographic location and nearness in time. The connections are stored in a database
that will relate the row numbers in the cluster database to a fire number or some similar key in the
time series database.

Once this program is complete, the data it creates can be queried to produce a time series of fire
power for a given fire.

## Libraries

### GLIB
So far I'm only using the following from GLIB:
 - The test module.
 - Command line option parser.
 - GArray
 - GList


### netcdf
 This is critical for accessing the data. The data files are in NetCDF4 format.


### SQLITE3
 sqlite3 is used for the database backend. In the future a different backend could be used for a
 more central storage that is accessable by multiple programs at a time, but for the time being 
 keeping the data on the same computer as the program will probably perform quicker and just be
 simpler.

### courier
 My own personal source library for a threadsafe queue for sending data between threads.

### kamel
 My own personal source library for writing KML files.
