use crate::modal_frame_spectrum::roundoff::partition::grid_log;
pub(super) use crate::modal_frame_spectrum::roundoff::partition::select_fine;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[path = "modal_frame_partition_tests.rs"]
mod tests;
