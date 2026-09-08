//! Cooperative control for synchronous, same-thread numerical calls.
//! Scopes restore on unwind; child threads must install their own explicit scope.
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SolverStage {
    LinearPrepare = 1,
    SparseIteration,
    DenseFactor,
    DenseSubstitution,
    BandedFactor,
    BandedSubstitution,
    TridiagonalFactor,
    TridiagonalSubstitution,
    ElementPrecompute,
    ElementAssembly,
    SparseCompress,
    PreconditionerSetup,
    IncompleteCholeskyFactor,
    IncompleteCholeskyTranspose,
    ConstraintIndex,
    ConstraintMap,
    ConstraintReduce,
    PreconditionerJacobi,
    SgsForward,
    SgsBackward,
    Ic0Forward,
    Ic0Backward,
    SparseMatvec,
    SparseMatvecRow,
    SparseResidual,
    SparseResidualRow,
    ResidualValidate,
    ResidualValidateRow,
    PcgRhsScale,
    PcgRhsNormalize,
    PcgDirectionCopy,
    PcgDot,
    PcgNorm,
    PcgVectorUpdate,
    PcgResidualUpdate,
    PcgDirectionUpdate,
    PcgSolutionScale,
}

impl SolverStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LinearPrepare => "linear_prepare",
            Self::SparseIteration => "sparse_iteration",
            Self::DenseFactor => "dense_factor",
            Self::DenseSubstitution => "dense_substitution",
            Self::BandedFactor => "banded_factor",
            Self::BandedSubstitution => "banded_substitution",
            Self::TridiagonalFactor => "tridiagonal_factor",
            Self::TridiagonalSubstitution => "tridiagonal_substitution",
            Self::ElementPrecompute => "element_precompute",
            Self::ElementAssembly => "element_assembly",
            Self::SparseCompress => "sparse_compress",
            Self::PreconditionerSetup => "preconditioner_setup",
            Self::IncompleteCholeskyFactor => "ic0_factor",
            Self::IncompleteCholeskyTranspose => "ic0_transpose",
            Self::ConstraintIndex => "constraint_index",
            Self::ConstraintMap => "constraint_map",
            Self::ConstraintReduce => "constraint_reduce",
            Self::PreconditionerJacobi => "preconditioner_jacobi",
            Self::SgsForward => "sgs_forward",
            Self::SgsBackward => "sgs_backward",
            Self::Ic0Forward => "ic0_forward",
            Self::Ic0Backward => "ic0_backward",
            Self::SparseMatvec => "sparse_matvec",
            Self::SparseMatvecRow => "sparse_matvec_row",
            Self::SparseResidual => "sparse_residual",
            Self::SparseResidualRow => "sparse_residual_row",
            Self::ResidualValidate => "residual_validate",
            Self::ResidualValidateRow => "residual_validate_row",
            Self::PcgRhsScale => "pcg_rhs_scale",
            Self::PcgRhsNormalize => "pcg_rhs_normalize",
            Self::PcgDirectionCopy => "pcg_direction_copy",
            Self::PcgDot => "pcg_dot",
            Self::PcgNorm => "pcg_norm",
            Self::PcgVectorUpdate => "pcg_vector_update",
            Self::PcgResidualUpdate => "pcg_residual_update",
            Self::PcgDirectionUpdate => "pcg_direction_update",
            Self::PcgSolutionScale => "pcg_solution_scale",
        }
    }

    fn decode(value: u8) -> Option<Self> {
        [
            Self::LinearPrepare,
            Self::SparseIteration,
            Self::DenseFactor,
            Self::DenseSubstitution,
            Self::BandedFactor,
            Self::BandedSubstitution,
            Self::TridiagonalFactor,
            Self::TridiagonalSubstitution,
            Self::ElementPrecompute,
            Self::ElementAssembly,
            Self::SparseCompress,
            Self::PreconditionerSetup,
            Self::IncompleteCholeskyFactor,
            Self::IncompleteCholeskyTranspose,
            Self::ConstraintIndex,
            Self::ConstraintMap,
            Self::ConstraintReduce,
            Self::PreconditionerJacobi,
            Self::SgsForward,
            Self::SgsBackward,
            Self::Ic0Forward,
            Self::Ic0Backward,
            Self::SparseMatvec,
            Self::SparseMatvecRow,
            Self::SparseResidual,
            Self::SparseResidualRow,
            Self::ResidualValidate,
            Self::ResidualValidateRow,
            Self::PcgRhsScale,
            Self::PcgRhsNormalize,
            Self::PcgDirectionCopy,
            Self::PcgDot,
            Self::PcgNorm,
            Self::PcgVectorUpdate,
            Self::PcgResidualUpdate,
            Self::PcgDirectionUpdate,
            Self::PcgSolutionScale,
        ]
        .into_iter()
        .find(|stage| *stage as u8 == value)
    }
}

/// A diagnostic observation, not a resumable numerical checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolverCheckpoint {
    pub stage: SolverStage,
    pub completed_steps: u64,
}

#[derive(Debug, Default)]
struct State {
    requested: AtomicBool,
    interrupted: AtomicBool,
    checkpoint: AtomicU64,
}

/// A sticky, per-execution token. Use a new token for a new execution.
#[derive(Debug, Clone, Default)]
pub struct SolverControl(Arc<State>);

impl SolverControl {
    pub fn request_cancel(&self) {
        self.0.requested.store(true, Ordering::Release);
    }

    pub fn cancellation_requested(&self) -> bool {
        self.0.requested.load(Ordering::Acquire)
    }

    pub fn was_interrupted(&self) -> bool {
        self.0.interrupted.load(Ordering::Acquire)
    }

    pub fn last_checkpoint(&self) -> Option<SolverCheckpoint> {
        let value = self.0.checkpoint.load(Ordering::Acquire);
        Some(SolverCheckpoint {
            stage: SolverStage::decode(value as u8)?,
            completed_steps: value >> 8,
        })
    }

    fn observe(&self, point: SolverCheckpoint) {
        let value = (point.completed_steps.min(u64::MAX >> 8) << 8) | point.stage as u64;
        self.0.checkpoint.store(value, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
pub struct SolverCancelled {
    pub checkpoint: Option<SolverCheckpoint>,
}

impl std::fmt::Display for SolverCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.checkpoint {
            Some(point) => write!(
                f,
                "solver cancelled at {} after {} steps",
                point.stage.as_str(),
                point.completed_steps
            ),
            None => f.write_str("solver cancelled before numerical execution"),
        }
    }
}

impl std::error::Error for SolverCancelled {}

impl From<SolverCancelled> for String {
    fn from(error: SolverCancelled) -> Self {
        error.to_string()
    }
}

#[derive(Clone)]
struct Scope {
    control: SolverControl,
    observer: Option<Rc<dyn Fn(SolverCheckpoint)>>,
}

thread_local! {
    static SCOPES: RefCell<Vec<Scope>> = const { RefCell::new(Vec::new()) };
}

struct Restore(usize);
impl Drop for Restore {
    fn drop(&mut self) {
        SCOPES.with_borrow_mut(|scopes| scopes.truncate(self.0));
    }
}

/// Install control around one synchronous call. Cancellation discards partial success.
pub fn with_solver_control<T, E: From<SolverCancelled>>(
    control: &SolverControl,
    operation: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    scoped(control, None, operation)
}

/// The observer runs synchronously at safe points, outside the scope registry borrow.
/// Slow observers delay execution; this is not an asynchronous progress subscription.
pub fn with_solver_observer<T, E: From<SolverCancelled>>(
    control: &SolverControl,
    observer: impl Fn(SolverCheckpoint) + 'static,
    operation: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    scoped(control, Some(Rc::new(observer)), operation)
}

fn scoped<T, E: From<SolverCancelled>>(
    control: &SolverControl,
    observer: Option<Rc<dyn Fn(SolverCheckpoint)>>,
    operation: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let _restore = Restore(SCOPES.with_borrow_mut(|scopes| {
        let previous = scopes.len();
        scopes.push(Scope {
            control: control.clone(),
            observer,
        });
        previous
    }));
    cancellation_error().map_or(Ok(()), Err)?;
    let result = operation();
    // Do not replace an unrelated numerical error with a concurrently arriving request.
    if result.is_ok() || control.was_interrupted() {
        cancellation_error().map_or(Ok(()), Err)?;
    }
    result
}

fn cancellation_error() -> Option<SolverCancelled> {
    SCOPES.with_borrow(|scopes| {
        if !scopes
            .iter()
            .any(|scope| scope.control.cancellation_requested() || scope.control.was_interrupted())
        {
            return None;
        }
        for scope in scopes {
            scope.control.0.interrupted.store(true, Ordering::Release);
        }
        Some(SolverCancelled {
            checkpoint: scopes.last().and_then(|s| s.control.last_checkpoint()),
        })
    })
}

pub(crate) fn check_cancellation() -> Result<(), String> {
    cancellation_error().map_or(Ok(()), |error| Err(error.to_string()))
}

/// Batch short preparation steps without putting an atomic poll in every inner operation.
#[inline]
pub(crate) fn checkpoint_chunk(
    stage: SolverStage,
    completed: usize,
    total: usize,
) -> Result<(), String> {
    if completed % 64 == 0 || completed == total {
        checkpoint(stage, completed)
    } else {
        Ok(())
    }
}

pub(crate) fn checkpoint(stage: SolverStage, completed_steps: usize) -> Result<(), String> {
    check_cancellation()?;
    let point = SolverCheckpoint {
        stage,
        completed_steps: completed_steps as u64,
    };
    let observers = SCOPES.with_borrow(|scopes| {
        let mut observers = Vec::new();
        for scope in scopes {
            scope.control.observe(point);
            if let Some(observer) = &scope.observer {
                observers.push(observer.clone());
            }
        }
        observers
    });
    for observer in observers {
        observer(point);
    }
    check_cancellation()
}

#[cfg(test)]
#[path = "solver_control_tests.rs"]
mod tests;
