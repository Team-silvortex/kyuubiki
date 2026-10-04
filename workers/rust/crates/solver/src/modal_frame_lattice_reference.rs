pub(super) use crate::modal_frame_spectrum::roundoff::BlockFit;
use crate::modal_frame_spectrum::roundoff::coupled_pairs;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

#[path = "modal_frame_coupling_graph_tests.rs"]
mod graph_tests;

#[path = "modal_frame_partition_reference.rs"]
mod partition;
