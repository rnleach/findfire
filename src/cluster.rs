/*!
 * Types and functions for working with clusters.
 *
 * A cluster describes the aggregate properties of a connected group (or cluster) of FirePoint
 * objects.
 */

pub use cluster::Cluster;
pub use cluster_list::ClusterList;
pub use cluster_record::ClusterRecord;

mod cluster;
mod cluster_list;
mod cluster_record;
