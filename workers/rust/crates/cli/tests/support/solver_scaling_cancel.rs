use super::*;

#[test]
fn cancel_sparse_rhs_validation_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "sparse_validate_rhs",
        HEAT,
        heat_grid("validate-rhs", 40),
        1024,
    )
}

#[test]
fn cancel_sparse_matrix_validation_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "sparse_validate_matrix",
        HEAT,
        heat_grid("validate-matrix", 40),
        64,
    )
}

#[test]
fn cancel_sparse_diagonal_scaling_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "sparse_diagonal_scale",
        HEAT,
        heat_grid("scale-diagonal", 40),
        64,
    )
}

#[test]
fn cancel_sparse_rhs_scaling_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass("sparse_rhs_scale", HEAT, heat_grid("scale-rhs", 40), 1024)
}

#[test]
fn cancel_sparse_diagonal_magnitude_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "sparse_diagonal_magnitude",
        HEAT,
        heat_grid("diagonal-magnitude", 40),
        64,
    )
}

#[test]
fn cancel_sparse_solution_unscaling_then_reuse_the_job() -> Result<(), Box<dyn Error>> {
    cancel_pass(
        "sparse_solution_unscale",
        HEAT,
        heat_grid("unscale-solution", 40),
        1024,
    )
}

#[test]
fn disconnected_sparse_validation_releases_capacity_without_releasing_the_hold()
-> Result<(), Box<dyn Error>> {
    disconnected_pass("sparse_validate_rhs", 1024)
}

#[test]
fn disconnected_sparse_unscaling_does_not_export_a_partial_solution() -> Result<(), Box<dyn Error>>
{
    disconnected_pass("sparse_solution_unscale", 1024)
}
