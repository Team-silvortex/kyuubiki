pub(crate) fn is_indexed_chain(
    node_count: usize,
    edges: impl IntoIterator<Item = (usize, usize)>,
) -> bool {
    if node_count < 2 {
        return false;
    }

    let mut spans = vec![false; node_count - 1];
    let mut edge_count = 0;
    for (first, second) in edges {
        let (left, right) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        if right != left + 1 || left >= spans.len() || spans[left] {
            return false;
        }
        spans[left] = true;
        edge_count += 1;
    }

    edge_count == spans.len() && spans.into_iter().all(|present| present)
}

pub(crate) fn solve_with_prescribed(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
    prescribed: &[(usize, f64)],
) -> Result<Vec<f64>, String> {
    let node_count = diagonal.len();
    if node_count == 0
        || lower.len() + 1 != node_count
        || upper.len() + 1 != node_count
        || rhs.len() != node_count
    {
        return Err("1d chain solver received inconsistent tridiagonal dimensions".to_string());
    }

    let mut values = vec![0.0; node_count];
    let mut fixed = vec![false; node_count];
    for &(index, value) in prescribed {
        if index >= node_count {
            return Err("1d chain solver received an out-of-range prescribed value".to_string());
        }
        if fixed[index] && (values[index] - value).abs() > 1.0e-12 {
            return Err("1d chain solver received conflicting prescribed values".to_string());
        }
        fixed[index] = true;
        values[index] = value;
    }

    let mut cursor = 0;
    while cursor < node_count {
        if fixed[cursor] {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while cursor < node_count && !fixed[cursor] {
            cursor += 1;
        }
        solve_free_segment(
            start,
            cursor,
            diagonal,
            lower,
            upper,
            rhs,
            &mut values,
            &fixed,
        )?;
    }
    Ok(values)
}

fn solve_free_segment(
    start: usize,
    end: usize,
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
    values: &mut [f64],
    fixed: &[bool],
) -> Result<(), String> {
    let count = end - start;
    let mut diagonal_work = diagonal[start..end].to_vec();
    let mut rhs_work = rhs[start..end].to_vec();

    if start > 0 && fixed[start - 1] {
        rhs_work[0] -= lower[start - 1] * values[start - 1];
    }
    if end < values.len() && fixed[end] {
        rhs_work[count - 1] -= upper[end - 1] * values[end];
    }

    for local in 1..count {
        let global = start + local;
        let pivot = diagonal_work[local - 1];
        if pivot.abs() <= 1.0e-18 {
            return Err("1d chain solver encountered a zero pivot".to_string());
        }
        let factor = lower[global - 1] / pivot;
        diagonal_work[local] -= factor * upper[global - 1];
        rhs_work[local] -= factor * rhs_work[local - 1];
    }

    if diagonal_work[count - 1].abs() <= 1.0e-18 {
        return Err("1d chain solver encountered a zero pivot".to_string());
    }
    values[end - 1] = rhs_work[count - 1] / diagonal_work[count - 1];
    for local in (0..count - 1).rev() {
        let global = start + local;
        values[global] =
            (rhs_work[local] - upper[global] * values[global + 1]) / diagonal_work[local];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_indexed_chain, solve_with_prescribed};

    #[test]
    fn recognizes_a_contiguous_chain_only_once_per_span() {
        assert!(is_indexed_chain(4, [(0, 1), (2, 1), (2, 3)]));
        assert!(!is_indexed_chain(4, [(0, 1), (1, 2), (1, 2)]));
    }

    #[test]
    fn solves_segments_around_prescribed_values() {
        let values = solve_with_prescribed(
            &[1.0, 2.0, 2.0, 1.0],
            &[-1.0, -1.0, -1.0],
            &[-1.0, -1.0, -1.0],
            &[0.0, 0.0, 0.0, 0.0],
            &[(0, 0.0), (3, 3.0)],
        )
        .expect("tridiagonal system should solve");
        assert_eq!(values, vec![0.0, 1.0, 2.0, 3.0]);
    }
}
