use crate::modal_frame_spectrum::roundoff;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use kyuubiki_protocol::{SolveModalFrame2dRequest, SolveModalFrame2dResult};

// Stable assembly order for the bounded planar single-mode path, not a retry.
pub(super) struct NodeOrder {
    original: Vec<usize>,
    canonical: Vec<usize>,
}

impl NodeOrder {
    pub(super) fn prepare(input: &SolveModalFrame2dRequest) -> Result<Option<Self>, String> {
        if input.mode_count != Some(1) || input.nodes.len() > roundoff::MAX_DOFS + 1 {
            return Ok(None);
        }
        let Some(first) = input.nodes.first() else {
            return Ok(None);
        };
        // Keep axial recognition and general/mixed-restraint frame paths unchanged.
        if input
            .nodes
            .iter()
            .any(|node| !node.fix_x || node.fix_y != node.fix_rz || node.y != first.y)
        {
            return Ok(None);
        }
        let active: usize = input
            .nodes
            .iter()
            .map(|node| {
                usize::from(!node.fix_x) + usize::from(!node.fix_y) + usize::from(!node.fix_rz)
            })
            .sum();
        if !roundoff::eligible(active) {
            return Ok(None);
        }
        checkpoint(SolverStage::ElementPrecompute, 0)?;
        let mut order: Vec<_> = (0..input.nodes.len()).collect();
        order.sort_unstable_by(|&left, &right| {
            let a = &input.nodes[left];
            let b = &input.nodes[right];
            coordinate_cmp(a.x, b.x).then(coordinate_cmp(a.y, b.y))
        });
        // Coincident nodes have no unique geometric key; preserve their existing path.
        if order.windows(2).any(|pair| {
            let a = &input.nodes[pair[0]];
            let b = &input.nodes[pair[1]];
            a.x == b.x && a.y == b.y
        }) || order.iter().enumerate().all(|(i, &old)| i == old)
        {
            return Ok(None);
        }
        checkpoint(SolverStage::ElementPrecompute, input.nodes.len())?;
        let mut canonical = vec![0; order.len()];
        for (new, &old) in order.iter().enumerate() {
            canonical[old] = new;
        }
        Ok(Some(Self {
            original: order,
            canonical,
        }))
    }

    pub(super) fn index(&self, original: usize) -> usize {
        self.canonical[original]
    }

    pub(super) fn restore(
        self,
        mut result: SolveModalFrame2dResult,
    ) -> Result<SolveModalFrame2dResult, String> {
        for mode in &mut result.modes {
            let mut shape = vec![0.0; mode.shape.len()];
            for (new, &old) in self.original.iter().enumerate() {
                shape[3 * old..3 * old + 3].copy_from_slice(&mode.shape[3 * new..3 * new + 3]);
                checkpoint_chunk(SolverStage::ResultNodes, new + 1, self.original.len())?;
            }
            mode.shape = shape;
            // Recheck the public representation after restoring its summation order.
            mode.participation_norm = crate::modal_math::checked_shape_norm(&mode.shape)?;
            if (mode.participation_norm - 1.0).abs() > 1e-10 {
                return Err("restored planar modal shape lost its unit participation norm".into());
            }
        }
        for dof in &mut result.free_dofs {
            *dof = 3 * self.original[*dof / 3] + *dof % 3;
        }
        result.free_dofs.sort_unstable();
        checkpoint(SolverStage::ResultTotals, result.modes.len())?;
        Ok(result)
    }
}

fn coordinate_cmp(left: f64, right: f64) -> std::cmp::Ordering {
    // Signed zeros are the same geometric point, including duplicate-key detection.
    if left == right {
        std::cmp::Ordering::Equal
    } else {
        left.total_cmp(&right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request(points: &[[f64; 2]]) -> SolveModalFrame2dRequest {
        let nodes: Vec<_> = points
            .iter()
            .enumerate()
            .map(|(i, &[x, y])| {
                json!({"id":format!("node-{i}"), "x":x, "y":y,
                "fix_x":true, "fix_y":x==0.0 && y==0.0, "fix_rz":x==0.0 && y==0.0,
                "load_x":0.0, "load_y":0.0, "moment_z":0.0})
            })
            .collect();
        serde_json::from_value(json!({"nodes":nodes, "elements":[], "mode_count":1})).unwrap()
    }

    #[test]
    fn modal_public_node_order_distinct_geometry_maps_in_both_directions() {
        for points in [
            [[2.0, 0.0], [0.0, 0.0], [1.0, 0.0]],
            [[2.0, -0.0], [0.0, 0.0], [1.0, 0.0]],
        ] {
            let input = request(&points);
            let order = NodeOrder::prepare(&input).unwrap().unwrap();
            assert_eq!(order.original, [1, 2, 0]);
            assert_eq!(order.canonical, [2, 0, 1]);
            for (new, &old) in order.original.iter().enumerate() {
                assert_eq!(order.index(old), new);
            }
        }
        assert!(
            NodeOrder::prepare(&request(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn modal_public_node_order_coincident_signed_zero_points_preserve_existing_path() {
        let input = request(&[[-0.0, 1.0], [-0.0, 2.0], [0.0, 1.0], [0.0, 0.0]]);
        assert!(NodeOrder::prepare(&input).unwrap().is_none());
        let input = request(&[[1.0, 0.0], [0.0, 0.0], [1.0, -0.0]]);
        assert!(NodeOrder::prepare(&input).unwrap().is_none());
    }

    #[test]
    fn modal_public_node_order_does_not_expand_mode_dimension_or_node_budgets() {
        let mut input = request(&[[2.0, 0.0], [0.0, 0.0], [1.0, 0.0]]);
        for mode_count in [None, Some(0), Some(2)] {
            input.mode_count = mode_count;
            assert!(NodeOrder::prepare(&input).unwrap().is_none());
        }
        let mut input = request(&[[2.0, 0.0], [0.0, 0.0], [1.0, 0.0]]);
        input.nodes[0].fix_x = false;
        assert!(NodeOrder::prepare(&input).unwrap().is_none());
        input.nodes[0].fix_x = true;
        input.nodes[0].fix_rz = true;
        assert!(NodeOrder::prepare(&input).unwrap().is_none());
        assert!(
            NodeOrder::prepare(&request(&[[0.0, 2.0], [0.0, 0.0], [0.0, 1.0]]))
                .unwrap()
                .is_none()
        );
        let points: Vec<_> = (0..130).rev().map(|i| [i as f64, 0.0]).collect();
        assert!(NodeOrder::prepare(&request(&points)).unwrap().is_none());
        let points: Vec<_> = (0..258).rev().map(|i| [i as f64, 0.0]).collect();
        let mut input = request(&points);
        for node in &mut input.nodes {
            node.fix_y = true;
            node.fix_rz = true;
        }
        input.nodes[0].fix_y = false;
        input.nodes[1].fix_y = false;
        assert!(NodeOrder::prepare(&input).unwrap().is_none());
    }

    #[test]
    fn modal_public_node_order_cancellation_does_not_modify_input_or_block_replay() {
        use crate::solver_control::{SolverControl, with_solver_observer};
        let input = request(&[[2.0, 0.0], [0.0, 0.0], [1.0, 0.0]]);
        let unchanged = serde_json::to_vec(&input).unwrap();
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ElementPrecompute {
                    cancel.request_cancel();
                }
            },
            || NodeOrder::prepare(&input),
        )
        .err()
        .unwrap();
        assert!(error.contains("cancel"));
        assert_eq!(serde_json::to_vec(&input).unwrap(), unchanged);
        assert!(NodeOrder::prepare(&input).unwrap().is_some());
    }
}
