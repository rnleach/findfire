use super::*;
use std::ops::ControlFlow;

const RTREE_CHILDREN_PER_NODE: usize = 8;
const OVERLAP_FUDGE_FACTOR: f64 = 1.0e-5;

#[derive(Debug)]
enum RTreeNode {
    Node {
        bbox: BoundingBox,
        children_overlap: bool,
        children: Vec<RTreeNode>,
    },
    Leaf {
        bbox: BoundingBox,
        hilbert_num: u64,
        index: usize,
    },
}

impl RTreeNode {
    fn bounding_box(&self) -> BoundingBox {
        match self {
            Self::Node { bbox, .. } => *bbox,
            Self::Leaf { bbox, .. } => *bbox,
        }
    }

    fn max_hilbert_num(&self) -> u64 {
        match self {
            Self::Leaf { hilbert_num, .. } => *hilbert_num,
            Self::Node { children, .. } => children
                .iter()
                .map(|node| node.max_hilbert_num())
                .max()
                .unwrap_or(0),
        }
    }

    fn new_nodes(children: Vec<Self>) -> Self {
        let mut bbox = BoundingBox {
            ll: Coord {
                lat: f64::INFINITY,
                lon: f64::INFINITY,
            },
            ur: Coord {
                lat: -f64::INFINITY,
                lon: -f64::INFINITY,
            },
        };

        let mut children_overlap = false;
        for (i, child_box) in children.iter().map(|c| c.bounding_box()).enumerate() {
            bbox.ll.lat = bbox.ll.lat.min(child_box.ll.lat);
            bbox.ll.lon = bbox.ll.lon.min(child_box.ll.lon);
            bbox.ur.lat = bbox.ur.lat.max(child_box.ur.lat);
            bbox.ur.lon = bbox.ur.lon.max(child_box.ur.lon);

            // Only check if we haven't already found an overlap.
            if !children_overlap {
                for other_child_box in children
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i) // Don't check a box for overlap against itself!
                    .map(|(_, c)| c.bounding_box())
                {
                    if child_box.overlap(&other_child_box, OVERLAP_FUDGE_FACTOR) {
                        children_overlap = true;
                        break;
                    }
                }
            }
        }

        Self::Node {
            bbox,
            children,
            children_overlap,
        }
    }

    fn num_children(&self) -> usize {
        match self {
            Self::Node { children, .. } => children.len(),
            Self::Leaf { .. } => 1,
        }
    }

    /// Recursively apply `update` to objects in `data` that have bounding boxes that overlap
    /// `region`.
    ///
    /// ## Parameters
    ///
    /// data - is managed by the parent Hilbert2DRTreeView object. Indexes of `Self::Leaf{..}`
    /// nodes index into this slice.
    ///
    /// region - is the region of interest.
    ///
    /// update - is a `Fn` to apply to any items that overlap `region`. If it returns `Break(..)`,
    /// then further iteration should stop. If it returns `Continue(..)`, then iteration should
    /// carry on. If the `bool` in the return value is `true`, then something was updated and the
    /// bounding boxes need to be checked for expansion. The arguments to this function are a
    /// mutable reference to an item stored in the underlying array whose bounding box overlaps
    /// `region`, the index of that item in the underlying array, and the `user_data` passed in
    /// below.
    ///
    /// user_data - will be passed from call to call of `update` in case you want to accumulate
    /// some data while iterating. This behaves similar to the accumulator on the fold method of
    /// iterators.
    ///
    /// ## Returns
    ///
    /// The first element of the returned tuple indicates if anything was updated. The second is
    /// any value you may want to return from the iteration.
    fn foreach<T, V, F>(
        &mut self,
        data: &mut [T],
        region: &BoundingBox,
        mut update: F,
        user_data: V,
    ) -> (bool, ControlFlow<V, V>)
    where
        T: Geo,
        F: FnMut(&mut T, usize, V) -> (bool, ControlFlow<V, V>) + Copy,
    {
        if !self.bounding_box().overlap(region, OVERLAP_FUDGE_FACTOR) {
            return (false, ControlFlow::Continue(user_data));
        }

        match self {
            Self::Leaf { index, bbox, .. } => {
                let (updated, rv) = update(&mut data[*index], *index, user_data);

                if updated {
                    *bbox = data[*index].bounding_box();
                }
                (updated, rv)
            }
            Self::Node {
                children,
                children_overlap,
                bbox,
            } => {
                let mut user_data = user_data;
                let mut updated = false;
                let mut keep_going = true;
                for child in children.iter_mut() {
                    let (local_updated, rv) = child.foreach(data, region, update, user_data);

                    if local_updated {
                        let child_box = child.bounding_box();

                        bbox.ll.lat = bbox.ll.lat.min(child_box.ll.lat);
                        bbox.ll.lon = bbox.ll.lon.min(child_box.ll.lon);
                        bbox.ur.lat = bbox.ur.lat.max(child_box.ur.lat);
                        bbox.ur.lon = bbox.ur.lon.max(child_box.ur.lon);

                        updated = true;
                    }

                    match rv {
                        ControlFlow::Continue(value) => user_data = value,
                        ControlFlow::Break(value) => {
                            user_data = value;
                            keep_going = false;
                            break;
                        }
                    }
                }

                // Check to see if expanding has caused any children to overlap.
                if !*children_overlap && updated {
                    for (i, child_box) in children.iter().map(|c| c.bounding_box()).enumerate() {
                        for other_child_box in children
                            .iter()
                            .enumerate()
                            .filter(|(j, _)| i != *j)
                            .map(|(_, c)| c.bounding_box())
                        {
                            if child_box.overlap(&other_child_box, OVERLAP_FUDGE_FACTOR) {
                                *children_overlap = true;
                                break;
                            }
                        }

                        if *children_overlap {
                            break;
                        }
                    }
                }

                let user_data = if keep_going {
                    ControlFlow::Continue(user_data)
                } else {
                    ControlFlow::Break(user_data)
                };

                (updated, user_data)
            }
        }
    }

    fn get_indexes_of_potential_overlap(&self, buffer: &mut Vec<usize>) {
        match self {
            Self::Leaf { index, .. } => buffer.push(*index),
            Self::Node {
                children,
                children_overlap,
                ..
            } => {
                if *children_overlap {
                    for child in children {
                        child.get_indexes_of_potential_overlap(buffer);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Hilbert2DRTreeView<'a, T> {
    root: RTreeNode,
    hc: HilbertCurve,
    data: &'a mut [T],
}

impl<'a, T: Geo> Hilbert2DRTreeView<'a, T> {
    /// Build a view into the provided list.
    pub fn build_for(data: &'a mut [T], precomputed_domain: Option<BoundingBox>) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let data_domain = precomputed_domain.unwrap_or_else(|| Self::build_domain(data));

        let hc = HilbertCurve::new(16, data_domain);

        // Build the leaf nodes - level 0
        let mut leaves = Vec::with_capacity(data.len());
        for item in data.iter_mut().enumerate() {
            let (index, item) = item;
            let bbox = item.bounding_box();
            let hilbert_num = hc.translate_to_curve_distance(item.centroid());
            leaves.push(RTreeNode::Leaf {
                bbox,
                hilbert_num,
                index,
            });
        }

        // Sort the leaf nodes by Hilbert number. This is how we get locality for the parent nodes.
        leaves.sort_unstable_by_key(RTreeNode::max_hilbert_num);

        let mut num_level_nodes = leaves.len();
        let mut level_nodes = leaves;
        while num_level_nodes > 1 {
            let child_nodes = level_nodes;
            level_nodes = Vec::with_capacity(child_nodes.len() / RTREE_CHILDREN_PER_NODE + 1);

            let mut children = Vec::with_capacity(RTREE_CHILDREN_PER_NODE);
            for child_node in child_nodes.into_iter() {
                children.push(child_node);

                if children.len() == RTREE_CHILDREN_PER_NODE {
                    let node = RTreeNode::new_nodes(children);
                    level_nodes.push(node);
                    children = Vec::with_capacity(RTREE_CHILDREN_PER_NODE);
                }
            }

            if !children.is_empty() {
                let node = RTreeNode::new_nodes(children);
                level_nodes.push(node);
            }

            num_level_nodes = level_nodes.len();
        }

        debug_assert_eq!(level_nodes.len(), 1);
        let root = level_nodes.into_iter().next().unwrap();

        Some(Hilbert2DRTreeView { root, hc, data })
    }

    /// Apply a function to all elements with boundaries that overlap.
    ///
    /// If the closure returns `true`, then an element was updated and we need to update the upper
    /// levels of the bounding boxes.
    ///
    /// Returns `true` if `update` EVER returns `true`, that is if anything was ever updated.
    pub fn foreach<V, F>(&mut self, region: BoundingBox, user_data: V, update: F) -> V
    where
        F: FnMut(&mut T, usize, V) -> (bool, ControlFlow<V, V>) + Copy,
    {
        if self.root.num_children() > 0 {
            let (_, rv) = self.root.foreach(self.data, &region, update, user_data);
            match rv {
                ControlFlow::Break(value) => value,
                ControlFlow::Continue(value) => value,
            }
        } else {
            user_data
        }
    }

    /// Get the indexes of items which potentially overlap other items.
    pub fn indexes_of_potential_overlap(&self) -> Vec<usize> {
        let mut buffer = Vec::with_capacity(self.data.len() / 1_000);

        if self.root.num_children() > 0 {
            self.root.get_indexes_of_potential_overlap(&mut buffer);
        }

        buffer
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

#[derive(Debug)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HilbertCoord {
    x: u32,
    y: u32,
}

#[cfg(test)]
mod test {
    use super::Geo;
    use super::*;

    #[test]
    fn test_integer_coordinat_converstions() {
        let domain = BoundingBox {
            ll: Coord { lat: 0.0, lon: 0.0 },
            ur: Coord { lat: 1.0, lon: 1.0 },
        };

        let test_coords_i1 = [
            HilbertCoord { x: 0, y: 0 },
            HilbertCoord { x: 0, y: 1 },
            HilbertCoord { x: 1, y: 1 },
            HilbertCoord { x: 1, y: 0 },
        ];

        let test_coords_i2 = [
            HilbertCoord { x: 0, y: 0 },
            HilbertCoord { x: 1, y: 0 },
            HilbertCoord { x: 1, y: 1 },
            HilbertCoord { x: 0, y: 1 },
            HilbertCoord { x: 0, y: 2 },
            HilbertCoord { x: 0, y: 3 },
            HilbertCoord { x: 1, y: 3 },
            HilbertCoord { x: 1, y: 2 },
            HilbertCoord { x: 2, y: 2 },
            HilbertCoord { x: 2, y: 3 },
            HilbertCoord { x: 3, y: 3 },
            HilbertCoord { x: 3, y: 2 },
            HilbertCoord { x: 3, y: 1 },
            HilbertCoord { x: 2, y: 1 },
            HilbertCoord { x: 2, y: 0 },
            HilbertCoord { x: 3, y: 0 },
        ];

        let test_dist = [0u64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

        let hc = HilbertCurve::new(1, domain);
        for h in 0..4 {
            let coords = hc.integer_to_coords(test_dist[h]);
            assert_eq!(coords, test_coords_i1[h]);

            let h2 = hc.coords_to_integer(test_coords_i1[h]);
            assert_eq!(h2, test_dist[h]);
        }

        let hc = HilbertCurve::new(2, domain);
        for h in 0..16 {
            let coords = hc.integer_to_coords(test_dist[h]);
            assert_eq!(coords, test_coords_i2[h]);

            let h2 = hc.coords_to_integer(test_coords_i2[h]);
            assert_eq!(h2, test_dist[h]);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test_domain_mapping(){
        let domain = BoundingBox{ll : Coord{lon: 0.0, lat: 0.0}, ur : Coord{lon: 1.0, lat: 1.0}};

        // Test values for the N=1 Hilbert curve on the unit square.
        let n1_pairs = [
            (Coord{lon: 0.25, lat: 0.25},  0u64),
            (Coord{lon: 0.25, lat: 0.75},  1),
            (Coord{lon: 0.75, lat: 0.75},  2),
            (Coord{lon: 0.75, lat: 0.25},  3),

            // Corners.
            (Coord{lon: 0.00, lat: 0.00},  0),
            (Coord{lon: 0.00, lat: 1.00},  1),
            (Coord{lon: 1.00, lat: 1.00},  2),
            (Coord{lon: 1.00, lat: 0.00},  3),
        ];

        let hc = HilbertCurve::new(1, domain);
        for i in 0..n1_pairs.len() {
            let hilbert_dist = n1_pairs[i].1;
            let coord = n1_pairs[i].0;

            let hd_calc = hc.translate_to_curve_distance(coord);

            assert_eq!(hd_calc, hilbert_dist);
        }

        let domain = BoundingBox{ll: Coord{lon: 0.0, lat: 0.0}, ur: Coord{lon: 10.0, lat: 10.0}};

        // Test values for the N=1 Hilbert curve on a square with edges of 10.0
        let n1_pairs_b = [
            (Coord{lon:  2.5, lat: 2.5},  0),
            (Coord{lon:  2.5, lat: 7.5},  1),
            (Coord{lon:  7.5, lat: 7.5},  2),
            (Coord{lon:  7.5, lat: 2.5},  3),

            // Corners.
            (Coord{lon:  0.0, lat:  0.0},  0),
            (Coord{lon:  0.0, lat: 10.0},  1),
            (Coord{lon: 10.0, lat: 10.0},  2),
            (Coord{lon: 10.0, lat:  0.0},  3),
        ];

        let hc = HilbertCurve::new(1, domain);
        for i in 0..n1_pairs_b.len() {
            let hilbert_dist = n1_pairs_b[i].1;
            let coord = n1_pairs_b[i].0;

            let hd_calc = hc.translate_to_curve_distance(coord);

            assert_eq!(hd_calc, hilbert_dist);
        }

        let domain = BoundingBox{ll: Coord{lon: -2.0, lat: 5.0},
                                 ur: Coord{lon: 10.0, lat: 17.0}};

        // Test values for the N=2 Hilbert curve on a square with edges of 12.0
        let n2_pairs = [
            (Coord{lon:  -0.5, lat:  5.5},   0),
            (Coord{lon:   2.5, lat:  5.5},   1),
            (Coord{lon:   2.5, lat:  9.5},   2),
            (Coord{lon:  -0.5, lat:  9.5},   3),
            (Coord{lon:  -0.5, lat: 12.5},   4),
            (Coord{lon:  -0.5, lat: 15.5},   5),
            (Coord{lon:   2.5, lat: 15.5},   6),
            (Coord{lon:   2.5, lat: 12.5},   7),
            (Coord{lon:   5.5, lat: 12.5},   8),
            (Coord{lon:   5.5, lat: 15.5},   9),
            (Coord{lon:   8.5, lat: 15.5},  10),
            (Coord{lon:   8.5, lat: 12.5},  11),
            (Coord{lon:   8.5, lat:  9.5},  12),
            (Coord{lon:   5.5, lat:  9.5},  13),
            (Coord{lon:   5.5, lat:  5.5},  14),
            (Coord{lon:   8.5, lat:  5.5},  15),

            // Corners.
            (Coord{lon: -2.0, lat:  5.0},   0),
            (Coord{lon: -2.0, lat: 17.0},   5),
            (Coord{lon: 10.0, lat: 17.0},  10),
            (Coord{lon: 10.0, lat:  5.0},  15),
        ];

        let hc = HilbertCurve::new(2, domain);
        for i in 0..n2_pairs.len() {
            let hilbert_dist = n2_pairs[i].1;
            let coord = n2_pairs[i].0;

            let hd_calc = hc.translate_to_curve_distance(coord);

            assert_eq!(hd_calc, hilbert_dist);
        }
    }

    #[derive(Clone, Debug)]
    struct LabeledBB {
        rect: BoundingBox,
        label: String,
    }

    impl LabeledBB {
        fn new(min_x: u32, min_y: u32) -> Self {
            //
            // All rects have width & height of 1
            let label = format!("{}x{}", min_x, min_y);
            let rect = BoundingBox {
                ll: Coord {
                    lon: min_x as f64,
                    lat: min_y as f64,
                },
                ur: Coord {
                    lon: (min_x + 1) as f64,
                    lat: (min_y + 1) as f64,
                },
            };

            LabeledBB { rect, label }
        }
    }

    impl Geo for LabeledBB {
        fn centroid(&self) -> Coord {
            let lat = (self.rect.ll.lat + self.rect.ur.lat) / 2.0;
            let lon = (self.rect.ll.lon + self.rect.ur.lon) / 2.0;
            Coord { lat, lon }
        }

        fn bounding_box(&self) -> BoundingBox {
            self.rect
        }
    }

    fn create_rectangles_for_rtree_view_test() -> Vec<LabeledBB> {
        let mut rects = Vec::with_capacity(40);
        for i in 1..=15 {
            if i % 2 == 0 {
                continue;
            }
            for j in 1..=9 {
                if j % 2 == 0 {
                    continue;
                }
                rects.push(LabeledBB::new(i, j));
            }
        }

        rects
    }

    fn test_bb_for_hits(mut rectangles: &mut [LabeledBB], bbox: BoundingBox, num_hits: usize) {
        println!("Target Area: {} Expected Hits: {}", bbox, num_hits);

        // Create the view.
        let mut view = Hilbert2DRTreeView::build_for(&mut rectangles, None).unwrap();

        // Count the hits.
        let hits = view.foreach(bbox, 0, |labeled_rect, _rect_idx, hits_so_far| {
            println!("{:?} overlaps {:?}", bbox, labeled_rect);
            (false, ControlFlow::Continue(hits_so_far + 1))
        });

        assert_eq!(hits, num_hits);
    }

    #[test]
    #[rustfmt::skip]
    fn rtree_test_query_whole_domain() {
        let mut rectangles = create_rectangles_for_rtree_view_test();

        // Check the whole domain
        let whole_domain = 
            BoundingBox {ll: Coord { lat: 0.0, lon: 0.0 }, ur: Coord {lat: 20.0, lon: 20.0}};

        let len = rectangles.len();
        test_bb_for_hits(&mut rectangles, whole_domain, len);
    }

    #[test]
    #[rustfmt::skip]
    fn rtree_test_query() {
        let mut rectangles = create_rectangles_for_rtree_view_test();

        for rec in &rectangles {
            println!("{:?}", rec);
        }

        // Check for several sub-rectangles
        let test_pairs = [
          (BoundingBox {ll: Coord { lat:   0.0, lon:   0.0}, ur: Coord {lat:  4.5, lon:  4.5}}, 4),
          (BoundingBox {ll: Coord { lat:   0.0, lon:   0.0}, ur: Coord {lat:  5.5, lon:  5.5}}, 9),
          (BoundingBox {ll: Coord { lat: -10.0, lon: -10.0}, ur: Coord {lat:  5.5, lon:  5.5}}, 9),
          (BoundingBox {ll: Coord { lat:   5.5, lon:   7.5}, ur: Coord {lat:  7.5, lon:  9.5}}, 4),
          (BoundingBox {ll: Coord { lat:   8.5, lon:  14.5}, ur: Coord {lat: 99.0, lon: 99.0}}, 1),
          (BoundingBox {ll: Coord { lat:   3.0, lon:   0.0}, ur: Coord {lat:  4.5, lon:  4.5}}, 2),
          (BoundingBox {ll: Coord { lat:   0.0, lon:   3.0}, ur: Coord {lat:  4.5, lon:  4.5}}, 2),

          // Just graze the edges
          (BoundingBox {ll: Coord { lat:   4.0, lon:   4.0}, ur: Coord {lat:  5.0, lon:  5.0}}, 4),
          // Hit nothing!
          (BoundingBox {ll: Coord { lat:   4.1, lon:   4.1}, ur: Coord {lat:  4.9, lon:  4.9}}, 0),
        ];

        for (bb, num_hit) in test_pairs{
            test_bb_for_hits(&mut rectangles, bb, num_hit);
        }
    }
}
