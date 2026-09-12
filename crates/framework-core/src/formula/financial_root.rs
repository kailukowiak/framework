//! A bounded bracketed solver for return formulas. Scanning in log-rate space
//! covers rates close to -1 and very large positive rates without letting a
//! Newton step leave the financial domain. Multiple-root discovery is bounded:
//! among roots exposed by this fixed scan, the one nearest the guess in
//! log-rate space wins, with the lower rate breaking an exact tie.

const MIN_LOG_RATE: f64 = -18.0;
const MAX_LOG_RATE: f64 = 18.0;
// Each sample evaluates the complete cash-flow column. This deliberately
// finite grid bounds both root discovery and work for large imported frames.
const SCAN_STEPS: usize = 4096;
const MAX_BISECTIONS: usize = 160;

#[derive(Clone, Copy, Debug)]
pub(super) struct RootOutcome {
    pub root: f64,
    pub iterations: usize,
    pub bracket: (f64, f64),
    /// Signed-log normalized objective, not a currency-denominated NPV.
    pub residual: f64,
}

pub(super) fn solve(
    guess: f64,
    objective: impl Fn(f64) -> Option<f64>,
) -> Result<RootOutcome, &'static str> {
    if !guess.is_finite() || guess <= -1.0 {
        return Err("guess must be finite and greater than -1");
    }
    let guess_x = guess.ln_1p();
    let mut points = Vec::with_capacity(SCAN_STEPS + 4);
    points.push(MIN_LOG_RATE);
    points.push(0.0);
    if (MIN_LOG_RATE..=MAX_LOG_RATE).contains(&guess_x) {
        points.push(guess_x);
    }
    for index in 0..=SCAN_STEPS {
        points
            .push(MIN_LOG_RATE + (MAX_LOG_RATE - MIN_LOG_RATE) * index as f64 / SCAN_STEPS as f64);
    }
    points.sort_by(f64::total_cmp);
    points.dedup_by(|a, b| a.to_bits() == b.to_bits());

    let sampled: Vec<_> = points
        .into_iter()
        .filter_map(|x| {
            objective(x)
                .filter(|v| v.is_finite())
                .map(|value| (x, value))
        })
        .collect();
    if sampled.len() < 2 {
        return Err("cash-flow curve is not finite in the search domain");
    }
    if sampled.iter().all(|(_, value)| *value == 0.0) {
        return Err("cash flows make the return indeterminate");
    }

    let mut roots = Vec::new();
    for &(x, value) in &sampled {
        if value == 0.0 && isolated_exact_root(x, &objective) {
            roots.push(RootOutcome {
                root: x.exp_m1(),
                iterations: 0,
                bracket: (x.exp_m1(), x.exp_m1()),
                residual: value,
            });
        }
    }
    let mut refinement_failed = false;
    for pair in sampled.windows(2) {
        let ((left, left_value), (right, right_value)) = (pair[0], pair[1]);
        if left_value != 0.0 && right_value != 0.0 && left_value.signum() != right_value.signum() {
            match bisect(left, right, left_value, &objective) {
                Ok(root) => roots.push(root),
                Err(_) => refinement_failed = true,
            }
        }
    }
    roots.retain(|outcome| outcome.root.is_finite() && outcome.root > -1.0);
    roots.sort_by(|a, b| a.root.total_cmp(&b.root));
    roots
        .dedup_by(|a, b| (a.root - b.root).abs() <= 1e-10 * (1.0 + a.root.abs().max(b.root.abs())));
    let selected = roots.into_iter().min_by(|a, b| {
        let da = (a.root.ln_1p() - guess_x).abs();
        let db = (b.root.ln_1p() - guess_x).abs();
        da.total_cmp(&db).then_with(|| a.root.total_cmp(&b.root))
    });
    selected.ok_or(if refinement_failed {
        "return root candidates did not converge within the iteration limit"
    } else {
        "no return root was found in the bounded search domain"
    })
}

fn isolated_exact_root(x: f64, objective: &impl Fn(f64) -> Option<f64>) -> bool {
    const PROBE: f64 = 1e-5;
    [x - PROBE, x + PROBE].into_iter().all(|probe| {
        (MIN_LOG_RATE..=MAX_LOG_RATE).contains(&probe)
            && objective(probe).is_some_and(|value| value.is_finite() && value != 0.0)
    })
}

fn bisect(
    mut left: f64,
    mut right: f64,
    mut left_value: f64,
    objective: &impl Fn(f64) -> Option<f64>,
) -> Result<RootOutcome, &'static str> {
    for iteration in 1..=MAX_BISECTIONS {
        let middle = left + (right - left) / 2.0;
        let value =
            objective(middle).ok_or("cash-flow curve became non-finite while refining a root")?;
        if value == 0.0 || (right - left).abs() <= 2e-14 * (1.0 + middle.abs()) {
            return Ok(RootOutcome {
                root: middle.exp_m1(),
                iterations: iteration,
                bracket: (left.exp_m1(), right.exp_m1()),
                residual: value,
            });
        }
        if value.signum() == left_value.signum() {
            left = middle;
            left_value = value;
        } else {
            right = middle;
        }
    }
    Err("return root did not converge within the iteration limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_keeps_convergence_facts_for_future_trace_plumbing() {
        let outcome = solve(0.1, |x| Some(x.exp_m1() - 0.2)).unwrap();
        assert!((outcome.root - 0.2).abs() < 1e-12);
        assert!(outcome.iterations > 0);
        assert!(outcome.bracket.0 <= outcome.root && outcome.root <= outcome.bracket.1);
        assert!(outcome.residual.abs() < 1e-12);
    }
}
