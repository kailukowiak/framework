//! LP witnesses plus exact rational certification for binary-logit separation.
use crate::exact::{Rational, basis, dot, rational, solve};
use microlp::{ComparisonOp, OptimizationDirection, Problem};
use num_traits::{One, ToPrimitive, Zero};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separation {
    Complete,
    CompleteOrQuasi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionError {
    RankDeficient,
    Inconclusive,
    SolverLimit,
    SolverFailure,
}

fn run(problem: &Problem) -> Result<Option<microlp::Solution>, DetectionError> {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| problem.solve()))
        .map_err(|_| DetectionError::SolverFailure)?;
    match outcome {
        Ok(outcome) if outcome.is_optimal() => outcome
            .into_solution()
            .map(Some)
            .map_err(|_| DetectionError::SolverLimit),
        Ok(_) => Err(DetectionError::SolverLimit),
        Err(microlp::Error::Infeasible) => Ok(None),
        Err(_) => Err(DetectionError::SolverFailure),
    }
}

/// None means an *exactly certified* positive null-weight vector exists.
/// For Z_i=(2y_i-1)[1,x_i], w>0 and Z' w=0 exclude every direction with
/// Z beta>=0 and at least one strict margin. This certifies absence of both
/// complete and quasi-complete separation; full column rank is checked first.
pub(crate) fn detect(
    z: &[Vec<Rational>],
    budget: Duration,
) -> Result<Option<Separation>, DetectionError> {
    if budget.is_zero() {
        return Err(DetectionError::SolverLimit);
    }
    let n = z.len();
    let p = z[0].len();
    let rank = solve(z, &vec![Rational::zero(); n])
        .ok_or(DetectionError::Inconclusive)?
        .1;
    if rank < p {
        return Err(DetectionError::RankDeficient);
    }
    let transposed: Vec<Vec<_>> = (0..p)
        .map(|j| z.iter().map(|row| row[j].clone()).collect())
        .collect();
    let mut overlap = Problem::new(OptimizationDirection::Minimize);
    overlap.set_time_limit(budget);
    let weights: Vec<_> = (0..n)
        .map(|_| overlap.add_var(1.0, (1.0, f64::INFINITY)))
        .collect();
    for row in &transposed {
        overlap.add_constraint(
            weights
                .iter()
                .zip(row)
                .map(|(&v, a)| (v, a.to_f64().unwrap())),
            ComparisonOp::Eq,
            0.0,
        );
    }
    if let Some(solution) = run(&overlap)? {
        if weights.iter().any(|&v| !solution[v].is_finite()) {
            return Err(DetectionError::Inconclusive);
        }
        let mut candidate: Vec<_> = weights.iter().map(|&v| rational(solution[v])).collect();
        let residual: Vec<_> = transposed.iter().map(|row| -dot(row, &candidate)).collect();
        if let Some((correction, _)) = solve(&transposed, &residual) {
            for (w, adjustment) in candidate.iter_mut().zip(correction) {
                *w += adjustment;
            }
            if candidate.iter().all(|w| w > &Rational::zero())
                && transposed.iter().all(|row| dot(row, &candidate).is_zero())
            {
                return Ok(None);
            }
        }
    }
    separation_witness(z, budget)
}

fn classify(z: &[Vec<Rational>], beta: &[Rational]) -> Option<Separation> {
    let margins: Vec<_> = z.iter().map(|row| dot(row, beta)).collect();
    if margins.iter().any(|m| m < &Rational::zero()) || margins.iter().all(Zero::is_zero) {
        return None;
    }
    Some(if margins.iter().all(|m| m > &Rational::zero()) {
        Separation::Complete
    } else {
        Separation::CompleteOrQuasi
    })
}

fn separation_witness(
    z: &[Vec<Rational>],
    budget: Duration,
) -> Result<Option<Separation>, DetectionError> {
    let p = z[0].len();
    let mut problem = Problem::new(OptimizationDirection::Maximize);
    problem.set_time_limit(budget);
    let variables: Vec<_> = (0..p)
        .map(|j| {
            let objective: Rational = z.iter().map(|row| row[j].clone()).sum();
            problem.add_var(objective.to_f64().unwrap(), (-1.0, 1.0))
        })
        .collect();
    for row in z {
        problem.add_constraint(
            variables
                .iter()
                .zip(row)
                .map(|(&v, a)| (v, a.to_f64().unwrap())),
            ComparisonOp::Ge,
            0.0,
        );
    }
    let solution = run(&problem)?.ok_or(DetectionError::Inconclusive)?;
    if variables.iter().any(|&v| !solution[v].is_finite()) {
        return Err(DetectionError::Inconclusive);
    }
    let candidate: Vec<_> = variables.iter().map(|&v| rational(solution[v])).collect();
    if let Some(kind) = classify(z, &candidate) {
        return Ok(Some(kind));
    }
    // Recover an exact vertex from the approximate active set. Near-active
    // selection is only a proposal: every final margin is checked exactly.
    let mut active = Vec::new();
    let mut rhs = Vec::new();
    for row in z {
        if dot(row, &candidate).to_f64().unwrap().abs() < 1e-7 {
            active.push(row.clone());
            rhs.push(Rational::zero());
        }
    }
    for (j, value) in candidate.iter().enumerate() {
        for sign in [-1.0, 1.0] {
            if (value.to_f64().unwrap() - sign).abs() < 1e-7 {
                active.push(basis(p, j));
                rhs.push(rational(sign));
            }
        }
    }
    let beta = solve(&active, &rhs).ok_or(DetectionError::Inconclusive)?.0;
    classify(z, &beta)
        .map(Some)
        .ok_or(DetectionError::Inconclusive)
}

pub(crate) fn signed_design(x: &[Vec<f64>], y: &[f64], scales: &[f64]) -> Vec<Vec<Rational>> {
    x.iter()
        .zip(y)
        .map(|(row, &label)| {
            let sign = rational(2.0 * label - 1.0);
            std::iter::once(Rational::one())
                .chain(
                    row.iter()
                        .zip(scales)
                        .map(|(&v, &scale)| rational(v) / rational(scale)),
                )
                .map(|v| v * &sign)
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lp_interruption_is_an_error_even_if_an_incumbent_exists() {
        let mut problem = Problem::new(OptimizationDirection::Maximize);
        let a = problem.add_var(1.0, (0.0, 10.0));
        let b = problem.add_var(2.0, (0.0, 10.0));
        problem.add_constraint([(a, 1.0), (b, 1.0)], ComparisonOp::Le, 5.0);
        problem.set_time_limit(Duration::ZERO);
        assert!(matches!(run(&problem), Err(DetectionError::SolverLimit)));
    }

    #[test]
    fn tiny_positive_margins_cannot_be_rounded_into_overlap() {
        let x = vec![vec![-1.0], vec![-1e-14], vec![1e-14], vec![1.0]];
        let z = signed_design(&x, &[0., 0., 1., 1.], &[1.]);
        assert_ne!(detect(&z, Duration::from_secs(2)), Ok(None));
    }
}
