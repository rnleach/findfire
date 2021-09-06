use crate::firepoint::FirePoint;
use geo::{
    algorithm::{centroid::Centroid, concave_hull::ConcaveHull},
    line_string, point, polygon, Point, Polygon,
};
use std::iter::FromIterator;

/**
 * The aggregate properties of a connected group of FirePoint objects.
 */
#[derive(Clone, Debug)]
pub struct Cluster {
    /// Perimeter
    pub perimeter: Polygon<f64>,
    /// Centroid
    pub centroid: Point<f64>,
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    pub power: f64,
    /// The number of points that are in this cluster.
    pub count: i32,
}

impl Cluster {
    /**
     * Group FirePoint objects into clusters.
     *
     * FirePoint objects that are directly adjacent to each other are grouped into clusters where
     * each point is in direct contact with at least one other point in the cluster.
     *
     * #Arguments
     * points - an array of FirePoint objects.
     *
     * #Returns
     * An array of struct Cluster objects.
     */
    pub fn from_fire_points(mut points: Vec<FirePoint>) -> Vec<Self> {
        let mut clusters: Vec<Self> = vec![];
        let mut cluster_points: Vec<Point<f64>> = vec![];
        let mut cluster_index_coords: Vec<(isize, isize)> = vec![];

        const NULL_PT: FirePoint = FirePoint {
            x: 0,
            y: 0,
            power: f64::NAN,
            lats: [f64::NAN; 4],
            lons: [f64::NAN; 4],
        };

        for i in 0..points.len() {
            if points[i].x == 0 && points[i].y == 0 {
                continue;
            }

            let curr_pt = std::mem::replace(&mut points[i], NULL_PT);

            let mut count = 1;
            let mut power = curr_pt.power;

            curr_pt
                .lats
                .iter()
                .cloned()
                .zip(curr_pt.lons.iter().cloned())
                .map(|(lat, lon)| point!(x: lat, y: lon))
                .for_each(|pnt| cluster_points.push(pnt));

            cluster_index_coords.push((curr_pt.x, curr_pt.y));

            for j in (i + 1)..points.len() {
                // Skip NULL_PT values
                if points[j].x == 0 && points[j].y == 0 {
                    continue;
                }

                let mut in_cluster = false;
                for (x, y) in &cluster_index_coords {
                    let dx = (x - points[j].x).abs();
                    let dy = (y - points[j].y).abs();

                    if dx <= 1 && dy <= 1 {
                        in_cluster = true;
                        break;
                    }
                }

                if in_cluster {
                    let candidate = std::mem::replace(&mut points[j], NULL_PT);
                    count += 1;
                    power += candidate.power;

                    candidate
                        .lats
                        .iter()
                        .cloned()
                        .zip(candidate.lons.iter().cloned())
                        .map(|(lat, lon)| point!(x: lat, y: lon))
                        .for_each(|pnt| cluster_points.push(pnt));

                    cluster_index_coords.push((candidate.x, candidate.y));
                }
            }

            let multi_pnt = geo::MultiPoint::from_iter(cluster_points.iter().cloned());
            let perimeter = multi_pnt.concave_hull(2.0);
            let centroid = multi_pnt.centroid().unwrap();
            let curr_clust = Cluster {
                count,
                power,
                perimeter,
                centroid,
            };

            clusters.push(curr_clust);
            cluster_points.truncate(0);
            cluster_index_coords.truncate(0);
        }

        clusters
    }
}

impl Default for Cluster {
    fn default() -> Self {
        Cluster {
            count: 0,
            power: 0.0,
            perimeter: polygon![],
            centroid: point!(x: 0.0, y: 0.0),
        }
    }
}
