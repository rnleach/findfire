use crate::firepoint::FirePoint;

/**
 * The aggregate properties of a connected group of FirePoint objects.
 */
#[derive(Clone, Copy, Debug)]
pub struct Cluster {
    /// Average latitude of the points in the cluster.
    pub lat: f64,
    /// Average longitude of the points in the cluster.
    pub lon: f64,
    /// Total (sum) of the fire power of the points in the cluster in megawatts.
    pub power: f64,
    /// The distance from the cluster center to the farthest point in the cluster.
    pub radius: f64,
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
        let mut cluster_points: Vec<FirePoint> = vec![];

        const NULL_PT: FirePoint = FirePoint {
            x: 0,
            y: 0,
            power: f64::NAN,
            lat: f64::NAN,
            lon: f64::NAN,
        };

        for i in 0..points.len() {
            if points[i].x == 0 && points[i].y == 0 {
                continue;
            }

            let curr_pt = std::mem::replace(&mut points[i], NULL_PT);

            cluster_points.push(curr_pt);

            for j in (i + 1)..points.len() {
                if points[j].x == 0 && points[j].y == 0 {
                    continue;
                }

                let mut in_cluster = false;
                for a_point_in_cluster in &cluster_points {
                    let dx = (a_point_in_cluster.x - points[j].x).abs();
                    let dy = (a_point_in_cluster.y - points[j].y).abs();

                    if dx <= 1 && dy <= 1 {
                        in_cluster = true;
                        break;
                    }
                }

                if in_cluster {
                    let candidate = std::mem::replace(&mut points[j], NULL_PT);
                    cluster_points.push(candidate);
                }
            }

            let mut curr_clust = Cluster {
                count: 0,
                lat: 0.0,
                lon: 0.0,
                power: 0.0,
                radius: 0.0,
            };

            for pnt in &cluster_points {
                curr_clust.lat += pnt.lat;
                curr_clust.lon += pnt.lon;
                curr_clust.power += pnt.power;
                curr_clust.count += 1;
            }

            curr_clust.lat /= curr_clust.count as f64;
            curr_clust.lon /= curr_clust.count as f64;

            for pnt in &cluster_points {
                let gs_distance = crate::geo::great_circle_distance(
                    pnt.lat,
                    pnt.lon,
                    curr_clust.lat,
                    curr_clust.lon,
                );

                curr_clust.radius = curr_clust.radius.max(gs_distance);
            }

            clusters.push(curr_clust);
            cluster_points.truncate(0);
        }

        clusters
    }
}

impl Default for Cluster {
    fn default() -> Self {
        Cluster {
            lat: f64::NAN,
            lon: f64::NAN,
            power: 0.0,
            radius: 0.0,
            count: 0,
        }
    }
}
