#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpdPreconditioner {
    Jacobi,
    SymmetricGaussSeidel,
}

#[derive(Debug, Clone)]
pub struct SpdSolveOptions {
    pub preconditioner: SpdPreconditioner,
}

#[derive(Debug, Clone)]
pub(crate) struct SpdSolveProfile {
    pub solution: Vec<f64>,
    pub iterations: usize,
    pub residual_norm: f64,
}

impl Default for SpdSolveOptions {
    fn default() -> Self {
        Self {
            preconditioner: SpdPreconditioner::Jacobi,
        }
    }
}
