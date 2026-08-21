//! Conservative suggestions for ordinary pasted rectangles in a worksheet.
//!
//! These are candidates, not imports. A person still previews and chooses the
//! range. That lets this detector remain deliberately simple: populated runs
//! connect through adjacent rows, blank gutters split neighboring tables, and
//! low-density fragments are ignored. Defined Excel Tables are filtered by
//! the caller because they carry stronger author intent.

use calamine::{Data, Range};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DetectedRegion {
    pub start: (u32, u32),
    pub end: (u32, u32),
}

impl DetectedRegion {
    pub fn row_count(self) -> usize {
        (self.end.0 - self.start.0 + 1) as usize
    }

    pub fn column_count(self) -> usize {
        (self.end.1 - self.start.1 + 1) as usize
    }
}

#[derive(Clone, Copy)]
struct RowRun {
    row: u32,
    start: u32,
    end: u32,
}

impl RowRun {
    fn width(self) -> u32 {
        self.end - self.start + 1
    }

    fn connects(self, other: Self) -> bool {
        let overlap_start = self.start.max(other.start);
        let overlap_end = self.end.min(other.end);
        overlap_start <= overlap_end
            && (overlap_end - overlap_start + 1) * 2 >= self.width().min(other.width())
    }
}

struct Components {
    parent: Vec<usize>,
}

impl Components {
    fn add(&mut self) -> usize {
        let index = self.parent.len();
        self.parent.push(index);
        index
    }

    fn root(&mut self, index: usize) -> usize {
        if self.parent[index] != index {
            self.parent[index] = self.root(self.parent[index]);
        }
        self.parent[index]
    }

    fn connect(&mut self, left: usize, right: usize) {
        let left = self.root(left);
        let right = self.root(right);
        if left != right {
            self.parent[right] = left;
        }
    }
}

pub(super) fn detect_rectangular_regions(values: &Range<Data>) -> Vec<DetectedRegion> {
    let (Some(start), Some(end)) = (values.start(), values.end()) else {
        return Vec::new();
    };
    let mut runs = Vec::<RowRun>::new();
    let mut components = Components { parent: Vec::new() };
    let mut previous = Vec::<usize>::new();

    for row in start.0..=end.0 {
        let current = populated_runs(values, row, start.1, end.1)
            .into_iter()
            .map(|run| {
                let index = components.add();
                runs.push(run);
                for &previous_index in &previous {
                    if run.connects(runs[previous_index]) {
                        components.connect(index, previous_index);
                    }
                }
                index
            })
            .collect::<Vec<_>>();
        previous = current;
    }

    let mut bounds = HashMap::<usize, DetectedRegion>::new();
    for (index, run) in runs.iter().copied().enumerate() {
        let root = components.root(index);
        bounds
            .entry(root)
            .and_modify(|region| {
                region.start.0 = region.start.0.min(run.row);
                region.start.1 = region.start.1.min(run.start);
                region.end.0 = region.end.0.max(run.row);
                region.end.1 = region.end.1.max(run.end);
            })
            .or_insert(DetectedRegion {
                start: (run.row, run.start),
                end: (run.row, run.end),
            });
    }

    let mut regions = bounds
        .into_values()
        .filter(|region| is_table_shaped(values, *region))
        .collect::<Vec<_>>();
    regions.sort_by_key(|region| region.start);
    regions
}

fn populated_runs(values: &Range<Data>, row: u32, start: u32, end: u32) -> Vec<RowRun> {
    let mut runs = Vec::new();
    let mut run_start = None;
    let mut last_populated = None;
    for column in start..=end {
        if is_populated(values.get_value((row, column))) {
            run_start.get_or_insert(column);
            last_populated = Some(column);
        } else if let (Some(start), Some(last)) = (run_start, last_populated)
            && column - last > 1
        {
            if last - start + 1 >= 2 {
                runs.push(RowRun {
                    row,
                    start,
                    end: last,
                });
            }
            run_start = None;
            last_populated = None;
        }
    }
    if let (Some(start), Some(last)) = (run_start, last_populated)
        && last - start + 1 >= 2
    {
        runs.push(RowRun {
            row,
            start,
            end: last,
        });
    }
    runs
}

fn is_table_shaped(values: &Range<Data>, region: DetectedRegion) -> bool {
    if region.row_count() < 3 || region.column_count() < 2 {
        return false;
    }
    let mut populated = 0usize;
    let mut first_five_populated = 0usize;
    let sampled_rows = region.row_count().min(5);
    for row in region.start.0..=region.end.0 {
        for column in region.start.1..=region.end.1 {
            if is_populated(values.get_value((row, column))) {
                populated += 1;
                if row < region.start.0 + sampled_rows as u32 {
                    first_five_populated += 1;
                }
            }
        }
    }
    let area = region.row_count() * region.column_count();
    let sampled_area = sampled_rows * region.column_count();
    populated * 2 >= area && first_five_populated * 5 >= sampled_area * 3
}

fn is_populated(value: Option<&Data>) -> bool {
    match value {
        None | Some(Data::Empty) => false,
        Some(Data::String(value)) => !value.trim().is_empty(),
        Some(value) => !value.to_string().trim().is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Cell;

    #[test]
    fn finds_separate_rectangles_and_ignores_scattered_notes() {
        let mut cells = Vec::new();
        for row in 0..100 {
            for column in 0..13 {
                if !((row + column) % 11 == 0 && row > 4) {
                    cells.push(Cell::new((row, column), Data::Int((row + column) as i64)));
                }
            }
        }
        for row in 14..25 {
            for column in 15..19 {
                cells.push(Cell::new((row, column), Data::Int((row + column) as i64)));
            }
        }
        cells.push(Cell::new((5, 30), Data::String("reviewed".into())));
        cells.push(Cell::new((40, 26), Data::String("note".into())));
        let values = Range::from_sparse(cells);

        assert_eq!(
            detect_rectangular_regions(&values),
            [
                DetectedRegion {
                    start: (0, 0),
                    end: (99, 12),
                },
                DetectedRegion {
                    start: (14, 15),
                    end: (24, 18),
                },
            ]
        );
    }

    #[test]
    fn rejects_small_or_low_density_fragments() {
        let values = Range::from_sparse(vec![
            Cell::new((0, 0), Data::Int(1)),
            Cell::new((0, 1), Data::Int(2)),
            Cell::new((1, 0), Data::Int(3)),
            Cell::new((1, 1), Data::Int(4)),
            Cell::new((5, 5), Data::Int(5)),
            Cell::new((5, 6), Data::Int(6)),
            Cell::new((6, 5), Data::Int(7)),
            Cell::new((7, 6), Data::Int(8)),
        ]);

        assert!(detect_rectangular_regions(&values).is_empty());
    }
}
