//! Small exact arithmetic certificate checker, deliberately bounded by caller.
//!
//! LP results propose certificates. Only these rational calculations can accept
//! them, so floating solver tolerances never turn near separation into overlap.
use num_rational::BigRational;
use num_traits::{One, Zero};
pub(crate) type Rational = BigRational;

pub(crate) fn rational(value: f64) -> Rational {
    Rational::from_float(value).expect("input was checked finite")
}

pub(crate) fn dot(left: &[Rational], right: &[Rational]) -> Rational {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

/// Solve A x = b exactly, assigning zero to free variables. The returned rank
/// lets the caller distinguish nonidentifiable models before invoking the fit.
pub(crate) fn solve(matrix: &[Vec<Rational>], rhs: &[Rational]) -> Option<(Vec<Rational>, usize)> {
    let width = matrix.first()?.len();
    let mut rows: Vec<_> = matrix
        .iter()
        .zip(rhs)
        .map(|(a, b)| {
            let mut row = a.clone();
            row.push(b.clone());
            row
        })
        .collect();
    let mut pivots = Vec::new();
    for column in 0..width {
        let Some(pivot) = (pivots.len()..rows.len()).find(|&r| !rows[r][column].is_zero()) else {
            continue;
        };
        let index = pivots.len();
        rows.swap(index, pivot);
        let divisor = rows[index][column].clone();
        for value in &mut rows[index][column..] {
            *value /= &divisor;
        }
        let pivot_row = rows[index].clone();
        for (r, row) in rows.iter_mut().enumerate() {
            if r == index {
                continue;
            }
            let factor = row[column].clone();
            for c in column..=width {
                row[c] -= &factor * &pivot_row[c];
            }
        }
        pivots.push(column);
    }
    if rows
        .iter()
        .any(|row| row[..width].iter().all(Zero::is_zero) && !row[width].is_zero())
    {
        return None;
    }
    let mut answer = vec![Rational::zero(); width];
    for (row, &column) in pivots.iter().enumerate() {
        answer[column] = rows[row][width].clone();
    }
    Some((answer, pivots.len()))
}

pub(crate) fn basis(width: usize, index: usize) -> Vec<Rational> {
    (0..width)
        .map(|j| {
            if j == index {
                Rational::one()
            } else {
                Rational::zero()
            }
        })
        .collect()
}
