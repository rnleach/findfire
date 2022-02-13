use crate::{
    database::FireDatabaseClusterRow,
    geo::{BoundingBox, Coord, Geo},
    pixel::PixelList,
    satellite::Satellite,
};
use chrono::{DateTime, Duration, Utc};

/**
 * The aggregate properties of a temporally connected group of [Cluster](crate::Cluster) objects.
 *
 * While the Clusters that make up a fire may come from any [Sector](crate::Sector) of a satellite
 * scan, they must come from the same [Satellite](crate::Satellite) because of the difficulty
 * associated with the different map projections and parallax. Currently the geo-location of an
 * observed [Pixel](crate::Pixel) does not take parallax into account. While this is a neglibible
 * issue for low elevation locations considering the resolution of the satellites, for higher
 * elevations it can cause a significant error. Also, for each satellite, the data was reprojected
 * into the exact same projection each time. So every image from a given satellite has the exact
 * same Pixel locations on the Earth's surface. As a result, aggregating values for maximum power,
 * area, or temperature is straight forward. If we had to deal with Pixels from different satellites
 * that don't totally overlap, or only partially overlap, it's not straightforward at all how to
 * combine the properties of those Pixels into a common projection.
 */
#[derive(Debug, Clone)]
pub struct Fire {
    /// The scan start time of the first Cluster where this fire was detected.
    first_observed: DateTime<Utc>,
    /// The scan end time of the last Cluster where this fire was detected.
    last_observed: DateTime<Utc>,
    /// The centroid of all the combined Clusters that contributed to this fire.
    centroid: Coord,
    /// The power of the most powerful Cluster that was associated with this fire. Note that
    /// several clusters may be associated with a fire at any given scan time, but they might be
    /// spatially separated (e.g. on different ends of the original fire). The powers of those
    /// different Clusters are NOT combined to come up with a total power for the time. This
    /// represents the single most powerful Cluster aggregated into this fire.
    max_power: f64,
    /// The maximum temperature of any Pixel that was ever associated with this fire.
    max_temperature: f64,
    /// An unique ID number for this fire that will be used identify this fire in a database that
    /// will also be used to associate this fire with Clusters which are a part of it.
    id: u64,
    /// Each Pixel in this contains the maximum power, area, and temperature observed in it's
    /// area during the fire. Since all the data for each satellite is projected to a common grid
    /// before being published online, throughout the life of the fire the Pixels will perfectly
    /// overlap. This is kind of a composite of the properties of the fire over it's lifetime.
    area: PixelList,
    /// The satellite the Clusters that were a part of this fire were observed with.
    sat: Satellite,
}

impl Fire {
    /// Create a new wildfire.
    pub fn new(id: u64, initial: FireDatabaseClusterRow) -> Self {
        Fire {
            first_observed: initial.start,
            last_observed: initial.end,
            centroid: initial.centroid,
            max_power: initial.power,
            max_temperature: initial.max_temperature,
            id,
            area: initial.pixels,
            sat: initial.sat,
        }
    }

    /// Get the id number of the fire.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get the time this fire was first observed.
    pub fn first_observed(&self) -> DateTime<Utc> {
        self.first_observed
    }

    /// Get the last time this fire was observed.
    pub fn last_observed(&self) -> DateTime<Utc> {
        self.last_observed
    }

    /// Get the duration of this fire (as observed by the satellite
    pub fn duration(&self) -> Duration {
        self.last_observed - self.first_observed
    }

    /// Get the maximum power observed for this fire, megawatts.
    pub fn max_power(&self) -> f64 {
        self.max_power
    }

    /// Get the max fire temperature observed on this fire, Kelvin.
    pub fn max_temperature(&self) -> f64 {
        self.max_temperature
    }

    /// Get access to the pixels in the wildfire.
    pub fn pixels(&self) -> &PixelList {
        &self.area
    }

    /// Get the satellite this fire was observed from.
    pub fn satellite(&self) -> Satellite {
        self.sat
    }

    /// Update a wildfire by adding the information in this FireDatabaseClusterRow to it.
    pub fn update(&mut self, row: &FireDatabaseClusterRow) {
        debug_assert!(row.sat == self.sat);

        self.last_observed = row.end;
        self.max_power = self.max_power.max(row.power);
        self.max_temperature = self.max_temperature.max(row.max_temperature);

        self.area.max_merge(&row.pixels);
        self.centroid = self.area.centroid();
    }

    /// Merge two wildfires.
    fn merge_with(&mut self, right: &mut Self) {
        debug_assert_eq!(self.sat, right.sat);

        if self.area.len() < right.area.len() {
            std::mem::swap(&mut self.area, &mut right.area);
        }

        if right.first_observed < self.first_observed {
            self.first_observed = right.first_observed;
        }

        if right.last_observed > self.last_observed {
            self.last_observed = right.last_observed;
        }

        // MUST DO THIS BEFORE UPDATING CENTROID
        self.area.max_merge(&right.area);

        self.centroid = self.area.centroid();
        self.max_power = self.max_power.max(right.max_power);
        self.max_temperature = self.max_temperature.max(right.max_temperature);
    }
}

impl Geo for Fire {
    fn centroid(&self) -> Coord {
        self.centroid
    }

    fn bounding_box(&self) -> BoundingBox {
        unimplemented!()
    }
}

/// A list of [Fire] objects.
pub struct FireList(Vec<Fire>);

impl Default for FireList {
    fn default() -> Self {
        Self::new()
    }
}

impl FireList {
    /// Create a new, empty list.
    pub fn new() -> Self {
        FireList(vec![])
    }

    /// Add a fire to the list.
    pub fn add_fire(&mut self, fire: Fire) {
        self.0.push(fire)
    }

    /// Create a new fire and add it to the list.
    pub fn create_add_fire(&mut self, id: u64, cluster_row: FireDatabaseClusterRow) {
        self.add_fire(Fire::new(id, cluster_row))
    }

    /// Update the list with the provided cluster.
    ///
    /// Matches the cluster to a wildfire in the list and then updates that wildfire.
    ///
    /// # Returns
    ///
    /// `Some(clust)` if `clust` was not matched to a fire and used to update it. If the
    /// `clust` was consumed, then it returns `None`.
    pub fn update(&mut self, row: FireDatabaseClusterRow) -> Option<FireDatabaseClusterRow> {
        let cluster_pixels: &PixelList = &row.pixels;
        for fire_pixels in self.0.iter_mut().map(|f| &mut f.area) {
            if cluster_pixels.adjacent_to_or_overlaps(fire_pixels, 1.0e-5) {
                fire_pixels.max_merge(cluster_pixels);
                return None;
            }
        }

        Some(row)
    }

    /// Extend a fire list using another fire list, the `src` list is left empty.
    pub fn extend(&mut self, src: &mut Self) {
        self.0.append(&mut src.0)
    }

    /// Detect overlaps in the fires in the list and merge them together into a single fire.
    pub fn merge_fires(&mut self, merged_away: &mut Self) {
        let mut i = 0;
        let mut len = self.0.len();
        while i < len {
            // safe because i < len as checked by the while condition
            let ifire = unsafe { &mut *(self.0.get_unchecked_mut(i) as *mut Fire) };
            let mut j = i + 1;
            while j < len {
                // safe because i != j, j > i and j < len as checked by while condition
                let jfire = unsafe { &mut *(self.0.get_unchecked_mut(j) as *mut Fire) };
                if ifire.area.adjacent_to_or_overlaps(&jfire.area, 1.0e-5) {
                    ifire.merge_with(jfire);
                    let temp = self.0.swap_remove(j);
                    len -= 1;
                    merged_away.0.push(temp);
                } else {
                    j += 1;
                }
            }

            i += 1;
        }
    }

    /// Get the number of fires in the list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Remove fires from the list that are likely no longer burning.
    ///
    /// # Arguments
    /// removed - is the list to add the drained elements into.
    /// current_time - is the current time of the clusters that are being processed.
    pub fn drain_stale_fires(&mut self, removed: &mut Self, current_time: DateTime<Utc>) {
        let mut i = 0;
        let mut len = self.0.len();
        while i < len {
            let f = unsafe { self.0.get_unchecked(i) };
            if wildfire_is_stale(f, current_time) {
                let temp = self.0.swap_remove(i);
                len -= 1;
                removed.0.push(temp);
            } else {
                i += 1;
            }
        }
    }
}

fn wildfire_is_stale(fire: &Fire, current_time: DateTime<Utc>) -> bool {
    let duration_since_last_observed = current_time - fire.last_observed;
    let wildfire_duration = fire.duration();

    if duration_since_last_observed < Duration::days(4) {
        // Give it at least four days to come back to life again.
        false
    } else if duration_since_last_observed > Duration::days(30)
        || wildfire_duration < duration_since_last_observed
    {
        // If it's been out for 30 days, it's stale OR
        // If it's not been seen in a longer time than it was burning, call it stale.
        true
    } else {
        false
    }
}
