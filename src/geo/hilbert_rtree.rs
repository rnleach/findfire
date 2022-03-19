use super::*;
use std::marker::PhantomData;

pub struct Hilbert2DRTreeView<'a, T> {
    leaves: Vec<RTreeLeaf>,
    nodes: Vec<RTreeNode>,
    hc: HilbertCurve,
    dummy: PhantomData<&'a [T]>,
}

impl<'a, T: Geo> Hilbert2DRTreeView<'a, T> {
    /// Build a view into the provided list.
    pub fn build_for(data: &'a [T], precomputed_domain: Option<BoundingBox>) -> Self {
        let data_domain = precomputed_domain.unwrap_or_else(|| Self::build_domain(data));

        let hc = HilbertCurve::new(16, data_domain);

        // Build the leaf nodes
        let mut leaves = Vec::with_capacity(data.len());
        for item in data.into_iter().enumerate() {
            let (index, item) = item;
            let bbox = item.bounding_box();
            let hilbert_num = hc.translate_to_curve_distance(item.centroid());
            leaves.push(RTreeLeaf {
                index,
                hilbert_num,
                bbox,
            });
        }

        // Sort the leaf nodes by Hilbert number. This is how we get locality for the parent nodes.
        leaves.sort_unstable_by_key(|a| a.hilbert_num);

        // Calculate the number of RTreeNode objects we'll need.
        let num_nodes_with_leaf_children = if data.len() % RTREE_CHILDREN_PER_NODE > 0 {
            data.len() / RTREE_CHILDREN_PER_NODE + 1
        } else {
            data.len() / RTREE_CHILDREN_PER_NODE
        };
        let mut num_nodes = num_nodes_with_leaf_children;
        let mut level_nodes = num_nodes;
        while level_nodes > 1 {
            level_nodes = if level_nodes % RTREE_CHILDREN_PER_NODE > 0 {
                level_nodes / RTREE_CHILDREN_PER_NODE + 1
            } else {
                level_nodes / RTREE_CHILDREN_PER_NODE
            };
            num_nodes += level_nodes;
        }
        let num_nodes = num_nodes;

        let mut nodes = vec![RTreeNode::default(); num_nodes];

        //
        // Initialize the group nodes to point down the tree and have the correct minimum bounding
        // rectangles.
        //

        // Fill in the nodes whose children are leaves, call these level 1 nodes.
        // (Leaf nodes are level 0 nodes.)
        let first_level_1_node_index = num_nodes - num_nodes_with_leaf_children;
        let mut num_leaves_left_to_process = leaves.len();
        let mut next_leaf = 0;
        for i in first_level_1_node_index..num_nodes {
            // Safe because we forced i to be less than num_nodes above.
            let node: &mut RTreeNode = unsafe { nodes.get_unchecked_mut(i) };

            let num_leaves_to_process = if num_leaves_left_to_process < RTREE_CHILDREN_PER_NODE {
                num_leaves_left_to_process
            } else {
                RTREE_CHILDREN_PER_NODE
            };

            debug_assert!(num_leaves_to_process > 0);

            // Initialize the minimum bounding rectangle to values that will certainly be
            // overwritten.
            node.bbox = BoundingBox {
                ll: Coord {
                    lat: f64::INFINITY,
                    lon: f64::INFINITY,
                },
                ur: Coord {
                    lat: -f64::INFINITY,
                    lon: -f64::INFINITY,
                },
            };

            node.children = RTreeNodeChildren::Leaf([0; 4]);
            for j in 0..num_leaves_to_process {
                match node.children {
                    RTreeNodeChildren::Leaf(ref mut leaves) => unsafe {
                        //leaves[j] = next_leaf;
                        *(leaves.get_unchecked_mut(j)) = next_leaf;
                    },
                    _ => unreachable!(),
                }

                let leaf: &RTreeLeaf = unsafe { leaves.get_unchecked_mut(next_leaf) };
                node.bbox.ll.lat = node.bbox.ll.lat.min(leaf.bbox.ll.lat);
                node.bbox.ll.lon = node.bbox.ll.lon.min(leaf.bbox.ll.lon);
                node.bbox.ur.lat = node.bbox.ur.lat.max(leaf.bbox.ur.lat);
                node.bbox.ur.lon = node.bbox.ur.lon.max(leaf.bbox.ur.lon);

                next_leaf += 1;
                num_leaves_left_to_process -= 1;
            }
            node.num_children = num_leaves_to_process;
        }

        // Fill in the nodes whose children are not leaves
        level_nodes = num_nodes_with_leaf_children;
        let mut num_filled_so_far = num_nodes_with_leaf_children;
        while num_filled_so_far < num_nodes {
            let num_children_at_level_below = level_nodes;
            let num_children_left_to_process = num_children_at_level_below;

            level_nodes = if level_nodes % RTREE_CHILDREN_PER_NODE > 0 {
                level_nodes / RTREE_CHILDREN_PER_NODE + 1
            } else {
                level_nodes / RTREE_CHILDREN_PER_NODE
            };

            let first_node_at_level = num_nodes - num_filled_so_far - level_nodes;
            for i in first_node_at_level..(num_nodes - num_filled_so_far) {
                let node: &mut RTreeNode =
                    unsafe { &mut *(nodes.get_unchecked_mut(i) as *mut RTreeNode) };

                let num_to_fill = if num_children_left_to_process < RTREE_CHILDREN_PER_NODE {
                    num_children_left_to_process
                } else {
                    RTREE_CHILDREN_PER_NODE
                };

                debug_assert!(num_to_fill > 0);

                // Initialize the minimum bounding rectangle to values that will certainly be
                // overwritten.
                node.bbox = BoundingBox {
                    ll: Coord {
                        lat: f64::INFINITY,
                        lon: f64::INFINITY,
                    },
                    ur: Coord {
                        lat: -f64::INFINITY,
                        lon: -f64::INFINITY,
                    },
                };

                node.children = RTreeNodeChildren::Node([0; 4]);
                for j in 0..num_to_fill {
                    let child_index = first_node_at_level
                        + level_nodes
                        + (i - first_node_at_level) * RTREE_CHILDREN_PER_NODE
                        + j;

                    match node.children {
                        RTreeNodeChildren::Node(ref mut children) => unsafe {
                            *(children.get_unchecked_mut(j)) = child_index;
                        },
                        _ => unreachable!(),
                    }

                    let child_node: &RTreeNode =
                        unsafe { &mut *(nodes.get_unchecked_mut(child_index) as *mut RTreeNode) };
                    node.bbox.ll.lat = node.bbox.ll.lat.min(child_node.bbox.ll.lat);
                    node.bbox.ll.lon = node.bbox.ll.lon.min(child_node.bbox.ll.lon);
                    node.bbox.ur.lat = node.bbox.ur.lat.max(child_node.bbox.ur.lat);
                    node.bbox.ur.lon = node.bbox.ur.lon.max(child_node.bbox.ur.lon);
                }
                node.num_children = num_to_fill;
            }

            num_filled_so_far += level_nodes;
        }

        Hilbert2DRTreeView {
            leaves,
            nodes,
            hc,
            dummy: PhantomData,
        }
    }

    /// Get a list of indexes for items that overlap the provided BoundingBox
    pub fn get_indexes_of_overlapping_items(&self, area: BoundingBox, buffer: &mut Vec<usize>) {
        buffer.clear();
        self.get_indexes_of_overlapping_children(0, area, buffer);
    }

    fn get_indexes_of_overlapping_children(
        &self,
        idx: usize,
        bbox: BoundingBox,
        buffer: &mut Vec<usize>,
    ) {
        let node: &RTreeNode = unsafe { self.nodes.get_unchecked(idx) };

        if node.bbox.overlap(&bbox, 1.0e-5) {
            match node.children {
                RTreeNodeChildren::Leaf(ref child_leaves) => {
                    for i in 0..node.num_children {
                        let leaf: &RTreeLeaf =
                            unsafe { self.leaves.get_unchecked(child_leaves[i]) };
                        if leaf.bbox.overlap(&bbox, 1.0e-5) {
                            buffer.push(leaf.index);
                        }
                    }
                }
                RTreeNodeChildren::Node(ref nodes) => {
                    for i in 0..node.num_children {
                        let next_idx = nodes[i];
                        self.get_indexes_of_overlapping_children(next_idx, bbox, buffer);
                    }
                }
                RTreeNodeChildren::Uninitialized => unreachable!(),
            }
        }
    }

    fn build_domain(data: &[T]) -> BoundingBox {
        let mut mbr = BoundingBox {
            ll: Coord {
                lat: f64::INFINITY,
                lon: f64::INFINITY,
            },
            ur: Coord {
                lat: -f64::INFINITY,
                lon: -f64::INFINITY,
            },
        };

        for item in data {
            let item_rect = item.bounding_box();

            mbr.ll.lat = mbr.ll.lat.min(item_rect.ll.lat);
            mbr.ll.lon = mbr.ll.lon.min(item_rect.ll.lon);
            mbr.ur.lat = mbr.ur.lat.max(item_rect.ur.lat);
            mbr.ur.lon = mbr.ur.lon.max(item_rect.ur.lon);
        }

        mbr
    }
}

const RTREE_CHILDREN_PER_NODE: usize = 4;

struct RTreeLeaf {
    index: usize,
    hilbert_num: u64,
    bbox: BoundingBox,
}

#[derive(Debug, Clone)]
enum RTreeNodeChildren {
    Leaf([usize; RTREE_CHILDREN_PER_NODE]), // Indexes into the leaves Vec
    Node([usize; RTREE_CHILDREN_PER_NODE]), // Indexes into the nodes Vec
    Uninitialized,                          // Not yet initialized
}

impl Default for RTreeNodeChildren {
    fn default() -> Self {
        RTreeNodeChildren::Uninitialized
    }
}

#[derive(Debug, Clone)]
struct RTreeNode {
    children: RTreeNodeChildren,
    num_children: usize,
    bbox: BoundingBox,
}

impl Default for RTreeNode {
    fn default() -> Self {
        RTreeNode {
            children: RTreeNodeChildren::default(),
            num_children: 0,
            bbox: BoundingBox::default(),
        }
    }
}

impl RTreeNode {}

struct HilbertCurve {
    // The number of iterations to use for this curve.
    //
    // This number can be a maximum of 31. If it is larger than 31, we won't have enough bits to do
    // the binary transformation correctly.
    iterations: u32,

    // This is the domain that the curve will cover.
    domain: BoundingBox,

    // These are needed for fast transformations from the "domain" space into the "Hilbert" space.
    max_dim: u32,
    width: f64,
    height: f64,
}

impl HilbertCurve {
    fn calc_max_dim_for_iterations(iterations: u32) -> u32 {
        (1u32 << iterations) - 1u32
    }

    fn calc_max_num(&self) -> u64 {
        let iterations = u64::from(self.iterations);

        (1u64 << (2 * iterations)) - 1u64
    }

    fn new(iterations: u32, domain: BoundingBox) -> Self {
        // iterations must be in the range 1 to 31 inclusive
        assert!((1..=31).contains(&iterations));

        let max_dim = Self::calc_max_dim_for_iterations(iterations);
        let width = domain.ur.lon - domain.ll.lon;
        let height = domain.ur.lat - domain.ll.lat;

        assert!(width > 0.0 && height >= 0.0);

        Self {
            iterations,
            domain,
            max_dim,
            width,
            height,
        }
    }

    fn integer_to_coords(&self, hilbert_int: u64) -> HilbertCoord {
        debug_assert!(hilbert_int <= self.calc_max_num());

        let mut x: u32 = 0;
        let mut y: u32 = 0;

        // This is the "transpose" operation.
        for b in 0..self.iterations {
            let bb = u64::from(b);
            let x_mask = 1u64 << (2 * bb + 1);
            let y_mask = 1u64 << (2 * bb);

            let x_val: u32 = ((hilbert_int & x_mask) >> (bb + 1)) as u32;
            let y_val: u32 = ((hilbert_int & y_mask) >> (bb)) as u32;

            x |= x_val;
            y |= y_val;
        }

        // Gray decode
        let z = 2u32 << (self.iterations - 1);
        let mut t = y >> 1;

        y ^= x;
        x ^= t;

        // Undo excess work
        let mut q = 2;
        while q != z {
            let p = q - 1;

            if (y & q) != 0 {
                x ^= p;
            } else {
                t = (x ^ y) & p;
                x ^= t;
                y ^= t;
            }

            if (x & q) != 0 {
                x ^= p;
            } else {
                t = (x ^ x) & p;
                x ^= t;
                x ^= t;
            }
            q <<= 1;
        }

        debug_assert!(x <= Self::calc_max_dim_for_iterations(self.iterations));
        debug_assert!(y <= Self::calc_max_dim_for_iterations(self.iterations));

        HilbertCoord { x, y }
    }

    fn coords_to_integer(&self, HilbertCoord { x, y }: HilbertCoord) -> u64 {
        debug_assert!(x <= Self::calc_max_dim_for_iterations(self.iterations));
        debug_assert!(y <= Self::calc_max_dim_for_iterations(self.iterations));

        let mut x = x;
        let mut y = y;

        let m = 1u32 << (self.iterations - 1);

        // Inverse undo excess work
        let mut q = m;
        while q > 1 {
            let p = q - 1;
            if (x & q) != 0 {
                x ^= p;
            } else {
                let t = (x ^ x) & p;
                x ^= t;
                x ^= t;
            }

            if (y & q) != 0 {
                x ^= p;
            } else {
                let t = (x ^ y) & p;
                x ^= t;
                y ^= t;
            }
            q >>= 1;
        }

        // Gray encode
        y ^= x;
        let mut t = 0u32;
        q = m;
        while q > 1 {
            if (y & q) != 0 {
                t ^= q - 1;
            }
            q >>= 1;
        }

        x ^= t;
        y ^= t;

        // This is the transpose operation
        let mut hilbert_int = 0;
        for b in 0..self.iterations {
            let bb = u64::from(b);
            let xx = u64::from(x);
            let yy = u64::from(y);
            let x_val: u64 = (((1u64 << bb) & xx) >> bb) << (2 * bb + 1);
            let y_val: u64 = (((1u64 << bb) & yy) >> bb) << (2 * bb);

            hilbert_int |= x_val;
            hilbert_int |= y_val;
        }

        debug_assert!(hilbert_int <= self.calc_max_num());

        hilbert_int
    }

    fn translate_to_hilbert_coords(&self, coord: Coord) -> HilbertCoord {
        let hilbert_edge_len = (self.max_dim + 1) as f64;

        let mut x = ((coord.lon - self.domain.ll.lon) / self.width * hilbert_edge_len) as u32;
        let mut y = ((coord.lat - self.domain.ll.lat) / self.height * hilbert_edge_len) as u32;

        x = x.min(self.max_dim);
        y = y.min(self.max_dim);

        HilbertCoord { x, y }
    }

    fn translate_to_curve_distance(&self, coord: Coord) -> u64 {
        let hilbert_coords = self.translate_to_hilbert_coords(coord);
        self.coords_to_integer(hilbert_coords)
    }
}

struct HilbertCoord {
    x: u32,
    y: u32,
}
