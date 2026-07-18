#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpdPreconditioner {
    IncompleteCholesky,
    Jacobi,
    SymmetricGaussSeidel,
}

#[derive(Debug, Clone)]
pub struct SpdSolveOptions {
    pub preconditioner: SpdPreconditioner,
    pub progress_interval: Option<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct SpdSolveProfile {
    pub solution: Vec<f64>,
    pub iterations: usize,
    pub matrix_non_zero_count: usize,
    pub residual_norm: f64,
    pub stages: Vec<SpdSolveStage>,
}

#[derive(Debug, Clone)]
pub(crate) struct SpdSolveStage {
    pub label: &'static str,
    pub elapsed_ms: f64,
}

impl Default for SpdSolveOptions {
    fn default() -> Self {
        Self {
            preconditioner: SpdPreconditioner::Jacobi,
            progress_interval: None,
        }
    }
}
