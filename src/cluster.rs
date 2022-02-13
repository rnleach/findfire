use crate::{
    end_time_from_file_name,
    firesatimage::{FirePoint, SatFireImage},
    geo::{BoundingBox, Coord, Geo},
    pixel::PixelList,
    satellite::{Satellite, Sector},
    start_time_from_file_name,
};
use chrono::{DateTime, Utc};
use std::{error::Error, path::Path};

/** Represents a spatially contiguous cluster of [Pixel](crate::Pixel) objects.
 *
 * This also contains the aggregate properties of the cluster such as the total power of all the
 * constituent Pixels, the total area, and the temperature of the hottest Pixel in the cluster.
 *
 * It should be noted that a power, area, and temperature are not analyzed for every Pixel, so
 * the aggregate properties only aggregate these paramters for the Pixels that report values.
 */
#[derive(Debug, Clone)]
pub struct Cluster {
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    power: f64,
    /// Total (sum) of the fire area of the points in the cluster with area in square meters.
    area: f64,
    /// Maximum temperature of all the pixels in the cluster in Kelvin.
    max_temp: f64,
    /// The maximum scan angle of any point in this cluster
    max_scan_angle: f64,
    /// Pixels making up the cluster.
    pixels: PixelList,
}

impl Default for Cluster {
    fn default() -> Self {
        Cluster {
            power: 0.0,
            area: 0.0,
            max_temp: 0.0,
            max_scan_angle: 0.0,
            pixels: PixelList::new(),
        }
    }
}

impl Cluster {
    /// Create a new Cluster with already initialized values.
    pub fn new(
        power: f64,
        area: f64,
        max_temp: f64,
        max_scan_angle: f64,
        pixels: PixelList,
    ) -> Cluster {
        Cluster {
            power,
            area,
            max_temp,
            max_scan_angle,
            pixels,
        }
    }

    /// Get the total power of all pixels in the Cluster, megawatts.
    pub fn total_power(&self) -> f64 {
        self.power
    }

    /// Get the total fire area of all pixels in the Cluster that had an area in the file, square
    /// meters.
    pub fn total_area(&self) -> f64 {
        self.area
    }

    /// Get the max fire temperature of all pixels in the Cluster that had a temperature in the
    /// file, Kelvin.
    pub fn max_temperature(&self) -> f64 {
        self.max_temp
    }

    /// Get the max scan angle of any pixel in this cluster.
    pub fn max_scan_angle(&self) -> f64 {
        self.max_scan_angle
    }

    /// Get the number of SFPixels in a Cluster.
    pub fn pixel_count(&self) -> usize {
        self.pixels.len()
    }

    /// Get access to the pixels in the cluster.
    pub fn pixels(&self) -> &PixelList {
        &self.pixels
    }

    /// Add a fire point to this Cluster.
    fn add_fire_point(&mut self, fire_point: FirePoint) {
        let FirePoint { pixel, .. } = fire_point;
        self.pixels.push(pixel);

        if pixel.power.is_finite() {
            self.power += pixel.power;
        }

        if pixel.temperature.is_finite() {
            self.max_temp = self.max_temp.max(pixel.temperature);
        }

        if pixel.area.is_finite() {
            self.area += pixel.area;
        }

        self.max_scan_angle = self.max_scan_angle.max(pixel.scan_angle);
    }
}

impl Geo for Cluster {
    fn centroid(&self) -> Coord {
        self.pixels.centroid()
    }

    fn bounding_box(&self) -> BoundingBox {
        self.pixels.bounding_box()
    }
}

/** A collection of [Cluster](crate::Cluster) objects.
 *
 * This collection stores a list of Clusters that are related. Specifically, they all come from
 * the same scan, or image. That means they share common scan start and end times, they come from
 * the same satellite, and they come from the same scan sector.
 */
pub struct ClusterList {
    satellite: Satellite,
    sector: Sector,
    /// Start time of the scan.
    start: DateTime<Utc>,
    /// End time of the scan
    end: DateTime<Utc>,
    /// List of [Cluster] objects associated with the above metadata.
    clusters: Vec<Cluster>,
}

impl ClusterList {
    /// Get the name of the satellite.
    pub fn satellite(&self) -> Satellite {
        self.satellite
    }

    /// Get the satellite sector.
    pub fn sector(&self) -> Sector {
        self.sector
    }

    /// Get the start time of the scan.
    pub fn scan_start(&self) -> DateTime<Utc> {
        self.start
    }

    /// Get the end time of the scan.
    pub fn scan_end(&self) -> DateTime<Utc> {
        self.end
    }

    /// Get the Clusters.
    pub fn clusters(&self) -> &[Cluster] {
        &self.clusters
    }

    /// Take the Clusters.
    pub fn take_clusters(self) -> Vec<Cluster> {
        self.clusters
    }

    /// Filter the ClusterList to only include fires with their centroid in the BoundingBox.
    pub fn filter_box(&mut self, bounding_box: BoundingBox) {
        self.clusters.retain(|cluster| {
            let centroid = cluster.centroid();
            bounding_box.contains_coord(centroid, 0.0)
        })
    }

    /// Filter the ClusterList to only include fires with their maximum scan angle below a
    /// threshold value.
    pub fn filter_scan_angle(&mut self, max_scan_angle: f64) {
        self.clusters
            .retain(|cluster| cluster.max_scan_angle < max_scan_angle)
    }

    /// Filter the ClusterList to only include fires for which the provided filter function returns
    /// true.
    pub fn filter<F: FnMut(&Cluster) -> bool>(&mut self, filter_func: F) {
        self.clusters.retain(filter_func)
    }

    /// Get the number of items in the ClusterList.
    pub fn len(&self) -> usize {
        self.clusters.len()
    }

    /// Get the total fire power of all the clusters in this list.
    pub fn total_power(&self) -> f64 {
        self.clusters
            .iter()
            .fold(0.0, |acc, cluster| acc + cluster.power)
    }
    /// Analyze a file and return a ClusterList.
    ///
    /// The metadata is gleaned from the file name, so this program relies on the current naming
    /// conventions of the NOAA big data program.
    pub fn from_file<P: AsRef<Path>>(full_path: P) -> Result<ClusterList, Box<dyn Error>> {
        let path: &Path = full_path.as_ref();
        let fname = path
            .file_name()
            .ok_or_else(|| "No file name".to_string())?
            .to_string_lossy();

        let satellite = Satellite::string_contains_satellite(&fname)
            .ok_or_else(|| "No satellite".to_string())?;
        let sector =
            Sector::string_contains_sector(&fname).ok_or_else(|| "No sector".to_string())?;

        let start =
            start_time_from_file_name(&fname).ok_or_else(|| "No start time.".to_string())?;
        let end = end_time_from_file_name(&fname).ok_or_else(|| "No end time".to_string())?;

        let fdata = SatFireImage::open(path)?;
        let points = fdata.extract_fire_points()?;
        let clusters: Vec<Cluster> = clusters_from_fire_points(points);

        Ok(ClusterList {
            satellite,
            sector,
            start,
            end,
            clusters,
        })
    }
}

fn clusters_from_fire_points(mut points: Vec<FirePoint>) -> Vec<Cluster> {
    let mut clusters: Vec<Cluster> = vec![];
    let mut cluster_points: Vec<FirePoint> = Vec::with_capacity(20);

    for i in 0..points.len() {
        let fp = unsafe { points.get_unchecked_mut(i) };

        if fp.x == isize::MIN && fp.y == isize::MIN {
            continue;
        }

        cluster_points.push(*fp);

        fp.x = isize::MIN;
        fp.y = isize::MIN;

        for j in (i + 1)..points.len() {
            let candidate = unsafe { points.get_unchecked_mut(j) };

            if candidate.x == isize::MIN && candidate.y == isize::MIN {
                continue;
            }

            let mut adjacent = false;
            for cluster_point in &cluster_points {
                let dx = (cluster_point.x - candidate.x).abs();
                let dy = (cluster_point.y - candidate.y).abs();

                if dx <= 1 && dy <= 1 {
                    adjacent = true;
                    break;
                }
            }

            if adjacent {
                cluster_points.push(*candidate);
                candidate.x = isize::MIN;
                candidate.y = isize::MIN;
            }
        }

        let mut curr_clust = Cluster::default();
        cluster_points
            .drain(..)
            .for_each(|cp| curr_clust.add_fire_point(cp));
        clusters.push(curr_clust);
    }

    clusters
}
