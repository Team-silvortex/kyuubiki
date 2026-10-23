use crate::solver_control::SolverStage;

pub(super) fn solve_row(
    rhs: f64,
    coefficients: &[f64],
    solved: &[f64],
    pivot: f64,
) -> Result<f64, String> {
    if coefficients.len() != solved.len() {
        return Err("dense substitution dimensions must match".into());
    }
    crate::linear_substitution::solve_row(
        rhs,
        coefficients.iter().copied().zip(solved.iter().copied()),
        pivot,
        SolverStage::DenseSubstitutionRange,
    )
}
