# SatFire
A collection of programs for working with GOES-16/17 Fire Detection Characterstics files to analyze
wildfire detections.

## findfire
Analysis of GOES-16/17 (GOES-R/S) NetCDF4 Fire Detection Characteristics files.

Given a directory containing *-FDCC-*, *-FDCF-*, or *-FDCM-* files (Fire Detection Characteristics)
from GOES-16/17 satellites, this program will analyze them. The analysis finds clusters of pixels
that are connected. The clusters are then stored in a database with the name of the satellite, the
scan sector, the scan angle (see below), cluster centroid latitude and longitude, the total fire 
power of the cluster in megawatts, the maximum fire temperature, the total estimated fire area, and
a binary representation of the image pixels that make up the cluster. 

The scan angle is the distance of the centroid of a pixel or cluster from the respective satellites
nadir position on the earth. The distance is in degrees from the center of the satellite's view.

The binary representation stored in the database includes the 4 corner coordinates, the scan angle,
the fire temperature in kelvin, the fire area in square meters, the fire power in megawatts, and 
some data quality flags for each pixel in the cluster. This basically represents all of the original
data that was used to construct the cluster.

The findfire program relies on the file naming convention used by the NOAA Big Data initiative to
detect satellite name, sector, scan start, and scan end times. Later versions may use attributes in
the NetCDF4 to detect these properties internally.

## showclusters
Select clusters from the database created by findfire and output them in a KML format.

This is a command line application that will select clusters based on a given start time, end time,
and geographic bounding box and then output them in KML. 

## currentclusters
Select the clusters from the most recent satellite image given a satellite name and sector name.

This command line application will query the database created by findfire for the most recent image
given the satelltie and sector and produce a KML file with all the clusters.

## connectfire
Create a database with the necessary information to create time series of fires.

This program will scan the cluster database created by findfire and connect clusters from different
scan times together based on their geographic location and nearness in time. The connections are 
stored in a database that will relate the row numbers in the cluster database to a fire number or 
some similar key in the time series database.

Once this program is complete, the data it creates can be queried to produce a time series of fire
power for a given fire.

## showfires

Select fires from the database created by connectfire and output them in a KML format. 

This is a command line application that will select fires based on a given start time, end time,
and geographic bounding box and then output them in KML. It is analagous to showclusters, however, 
showclusters only selects clusters that are within the given start and end times, wheres showfires 
selects any fire that overlaps the start and end times.

