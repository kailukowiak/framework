//! Excel is an interchange source, never a second document model.
//!
//! We read one explicit rectangle, use cached formula answers as ordinary
//! values, and immediately normalize it into the same Parquet artifact every
//! other import uses. Nothing here parses, stores, or attempts to refresh an
//! Excel formula. That boundary is what keeps opening a workbook from becoming
//! a promise to emulate Excel.

use super::excel_regions::detect_rectangular_regions;
use crate::{CoreError, DataArtifact, create_data_artifact};
use calamine::{Data, Range, Reader, Xlsx, open_workbook};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;
use uuid::Uuid;

const EXCEL_MAX_ROWS: u32 = 1_048_576;
const EXCEL_MAX_COLUMNS: u32 = 16_384;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExcelTableInfo {
    pub name: String,
    pub sheet_name: String,
    pub cell_range: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExcelRegionInfo {
    pub sheet_name: String,
    pub cell_range: String,
    pub row_count: usize,
    pub column_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExcelSheetInfo {
    pub name: String,
    pub used_range: Option<String>,
    pub row_count: usize,
    pub column_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExcelWorkbookInfo {
    pub path: String,
    pub file_name: String,
    pub sheets: Vec<ExcelSheetInfo>,
    pub tables: Vec<ExcelTableInfo>,
    pub suggested_regions: Vec<ExcelRegionInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExcelRangePreview {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub row_count: usize,
    pub formula_cell_count: usize,
    pub error_cell_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellRange {
    start: (u32, u32),
    end: (u32, u32),
}

impl CellRange {
    fn width(self) -> usize {
        (self.end.1 - self.start.1 + 1) as usize
    }
}

pub fn inspect_excel_workbook(path: &Path) -> Result<ExcelWorkbookInfo, CoreError> {
    let mut workbook: Xlsx<_> = open_workbook(path).map_err(excel_error)?;
    let sheet_names = workbook.sheet_names();
    let mut sheets = Vec::with_capacity(sheet_names.len());
    let mut suggested_regions = Vec::new();
    for name in &sheet_names {
        let range = workbook.worksheet_range(name).map_err(excel_error)?;
        suggested_regions.extend(
            detect_rectangular_regions(&range)
                .into_iter()
                .map(|region| ExcelRegionInfo {
                    sheet_name: name.clone(),
                    cell_range: format_cell_range(CellRange {
                        start: region.start,
                        end: region.end,
                    }),
                    row_count: region.row_count(),
                    column_count: region.column_count(),
                }),
        );
        sheets.push(ExcelSheetInfo {
            name: name.clone(),
            used_range: range
                .start()
                .zip(range.end())
                .map(|(start, end)| format_cell_range(CellRange { start, end })),
            row_count: range.height(),
            column_count: range.width(),
        });
    }

    workbook.load_tables().map_err(excel_error)?;
    let table_names = workbook
        .table_names()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut tables = Vec::with_capacity(table_names.len());
    for name in table_names {
        let table = workbook.table_by_name(&name).map_err(excel_error)?;
        let data = table.data();
        if let (Some(start), Some(end)) = (data.start(), data.end()) {
            tables.push(ExcelTableInfo {
                name,
                sheet_name: table.sheet_name().to_string(),
                // Calamine exposes an Excel Table's body range without its
                // header. The picker promises a ready-to-import rectangle, so
                // put that header row back and keep `has_header = true`.
                cell_range: format_cell_range(table_range_with_header(start, end)),
            });
        }
    }
    // A defined Excel Table is a stronger declaration than a shape inferred
    // from its populated cells. Do not show the same rectangle twice.
    suggested_regions.retain(|region| {
        !tables.iter().any(|table| {
            table.sheet_name == region.sheet_name
                && parse_cell_range(&table.cell_range).is_ok_and(|table_range| {
                    parse_cell_range(&region.cell_range)
                        .is_ok_and(|candidate| substantially_overlap(candidate, table_range))
                })
        })
    });

    Ok(ExcelWorkbookInfo {
        path: path.display().to_string(),
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Workbook.xlsx")
            .to_string(),
        sheets,
        tables,
        suggested_regions,
    })
}

fn substantially_overlap(left: CellRange, right: CellRange) -> bool {
    let start_row = left.start.0.max(right.start.0);
    let end_row = left.end.0.min(right.end.0);
    let start_column = left.start.1.max(right.start.1);
    let end_column = left.end.1.min(right.end.1);
    if start_row > end_row || start_column > end_column {
        return false;
    }
    let intersection =
        (end_row - start_row + 1) as usize * (end_column - start_column + 1) as usize;
    let left_area = (left.end.0 - left.start.0 + 1) as usize * left.width();
    let right_area = (right.end.0 - right.start.0 + 1) as usize * right.width();
    intersection * 2 >= left_area.min(right_area)
}

fn table_range_with_header(start: (u32, u32), end: (u32, u32)) -> CellRange {
    CellRange {
        start: (start.0.saturating_sub(1), start.1),
        end,
    }
}

pub fn preview_excel_range(
    path: &Path,
    sheet_name: &str,
    cell_range: &str,
    has_header: bool,
    limit: usize,
) -> Result<ExcelRangePreview, CoreError> {
    let selection = parse_cell_range(cell_range)?;
    let mut workbook: Xlsx<_> = open_workbook(path).map_err(excel_error)?;
    let values = workbook.worksheet_range(sheet_name).map_err(excel_error)?;
    let formulas = workbook
        .worksheet_formula(sheet_name)
        .map_err(excel_error)?;
    preview_from_ranges(&values, &formulas, selection, has_header, limit)
}

pub fn create_excel_range_artifact(
    source_path: &Path,
    data_directory: &Path,
    sheet_name: &str,
    cell_range: &str,
    has_header: bool,
) -> Result<(DataArtifact, ExcelRangePreview), CoreError> {
    fs::create_dir_all(data_directory).map_err(|error| CoreError::Import(error.to_string()))?;
    let selection = parse_cell_range(cell_range)?;
    let mut workbook: Xlsx<_> = open_workbook(source_path).map_err(excel_error)?;
    let values = workbook.worksheet_range(sheet_name).map_err(excel_error)?;
    let formulas = workbook
        .worksheet_formula(sheet_name)
        .map_err(excel_error)?;
    let report = preview_from_ranges(&values, &formulas, selection, has_header, 20)?;
    if report.row_count == 0 {
        return Err(CoreError::Import(
            "The selected Excel range has a header but no data rows".into(),
        ));
    }

    let temporary = data_directory.join(format!(".excel-{}.csv", Uuid::new_v4()));
    let write_result = write_selection_csv(&temporary, &values, selection, has_header);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    let artifact_result = create_data_artifact(&temporary, data_directory);
    let _ = fs::remove_file(&temporary);
    let mut artifact = artifact_result?;
    let workbook_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Workbook.xlsx");
    artifact.source_name = format!(
        "{workbook_name} · {sheet_name}!{}",
        format_cell_range(selection)
    );
    Ok((artifact, report))
}

fn preview_from_ranges(
    values: &Range<Data>,
    formulas: &Range<String>,
    selection: CellRange,
    has_header: bool,
    limit: usize,
) -> Result<ExcelRangePreview, CoreError> {
    let columns = selection_headers(values, selection, has_header);
    let first_data_row = selection.start.0 + u32::from(has_header);
    let row_count = if first_data_row > selection.end.0 {
        0
    } else {
        (selection.end.0 - first_data_row + 1) as usize
    };
    let rows = (first_data_row..=selection.end.0)
        .take(limit)
        .map(|row| {
            (selection.start.1..=selection.end.1)
                .map(|column| cell_text(values.get_value((row, column))))
                .collect()
        })
        .collect();
    let mut formula_cell_count = 0;
    let mut error_cell_count = 0;
    for row in selection.start.0..=selection.end.0 {
        for column in selection.start.1..=selection.end.1 {
            if formulas
                .get_value((row, column))
                .is_some_and(|formula| !formula.trim().is_empty())
            {
                formula_cell_count += 1;
            }
            if matches!(values.get_value((row, column)), Some(Data::Error(_))) {
                error_cell_count += 1;
            }
        }
    }
    if columns.is_empty() {
        return Err(CoreError::Import(
            "The selected Excel range has no columns".into(),
        ));
    }
    Ok(ExcelRangePreview {
        columns,
        rows,
        row_count,
        formula_cell_count,
        error_cell_count,
    })
}

fn write_selection_csv(
    path: &Path,
    values: &Range<Data>,
    selection: CellRange,
    has_header: bool,
) -> Result<(), CoreError> {
    let file = fs::File::create(path).map_err(|error| CoreError::Import(error.to_string()))?;
    let mut writer = BufWriter::new(file);
    write_csv_row(
        &mut writer,
        &selection_headers(values, selection, has_header),
    )?;
    let first_data_row = selection.start.0 + u32::from(has_header);
    if first_data_row <= selection.end.0 {
        for row in first_data_row..=selection.end.0 {
            let cells = (selection.start.1..=selection.end.1)
                .map(|column| cell_text(values.get_value((row, column))))
                .collect::<Vec<_>>();
            write_csv_row(&mut writer, &cells)?;
        }
    }
    writer
        .flush()
        .map_err(|error| CoreError::Import(error.to_string()))
}

fn write_csv_row(writer: &mut impl Write, cells: &[String]) -> Result<(), CoreError> {
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",").map_err(io_import_error)?;
        }
        if cell.contains([',', '"', '\n', '\r']) {
            writer.write_all(b"\"").map_err(io_import_error)?;
            writer
                .write_all(cell.replace('"', "\"\"").as_bytes())
                .map_err(io_import_error)?;
            writer.write_all(b"\"").map_err(io_import_error)?;
        } else {
            writer.write_all(cell.as_bytes()).map_err(io_import_error)?;
        }
    }
    writer.write_all(b"\n").map_err(io_import_error)
}

fn selection_headers(values: &Range<Data>, selection: CellRange, has_header: bool) -> Vec<String> {
    let mut counts = HashMap::<String, usize>::new();
    (0..selection.width())
        .map(|offset| {
            let fallback = format!("Column {}", offset + 1);
            let base = if has_header {
                let value = cell_text(
                    values.get_value((selection.start.0, selection.start.1 + offset as u32)),
                );
                if value.trim().is_empty() {
                    fallback
                } else {
                    value
                }
            } else {
                fallback
            };
            let count = counts.entry(base.clone()).or_default();
            *count += 1;
            if *count == 1 {
                base
            } else {
                format!("{base} {}", *count)
            }
        })
        .collect()
}

fn cell_text(value: Option<&Data>) -> String {
    match value {
        None | Some(Data::Empty) => String::new(),
        Some(Data::DateTime(value)) if value.is_datetime() => {
            let (year, month, day, hour, minute, second, millis) = value.to_ymd_hms_milli();
            if hour == 0 && minute == 0 && second == 0 && millis == 0 {
                format!("{year:04}-{month:02}-{day:02}")
            } else if millis == 0 {
                format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
            } else {
                format!(
                    "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03}"
                )
            }
        }
        Some(value) => value.to_string(),
    }
}

fn parse_cell_range(input: &str) -> Result<CellRange, CoreError> {
    let input = input.trim();
    let mut parts = input.split(':');
    let start = parse_cell(parts.next().unwrap_or_default())?;
    let end = parts.next().map(parse_cell).transpose()?.unwrap_or(start);
    if parts.next().is_some() || start.0 > end.0 || start.1 > end.1 {
        return Err(CoreError::Import(format!(
            "Excel range must run from its top-left to bottom-right cell: {input}"
        )));
    }
    Ok(CellRange { start, end })
}

fn parse_cell(input: &str) -> Result<(u32, u32), CoreError> {
    let compact = input.trim().replace('$', "");
    let split = compact
        .find(|character: char| character.is_ascii_digit())
        .ok_or_else(|| CoreError::Import(format!("Excel cell needs a row number: {input}")))?;
    let (letters, digits) = compact.split_at(split);
    if letters.is_empty()
        || digits.is_empty()
        || !letters
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        || !digits.chars().all(|character| character.is_ascii_digit())
    {
        return Err(CoreError::Import(format!("Invalid Excel cell: {input}")));
    }
    let mut column = 0u32;
    for character in letters.chars() {
        column = column
            .checked_mul(26)
            .and_then(|value| {
                value.checked_add(character.to_ascii_uppercase() as u32 - 'A' as u32 + 1)
            })
            .ok_or_else(|| CoreError::Import(format!("Excel column is too large: {letters}")))?;
    }
    let row = digits
        .parse::<u32>()
        .map_err(|_| CoreError::Import(format!("Invalid Excel row: {digits}")))?;
    if row == 0 || row > EXCEL_MAX_ROWS || column == 0 || column > EXCEL_MAX_COLUMNS {
        return Err(CoreError::Import(format!(
            "Excel cell is outside the worksheet: {input}"
        )));
    }
    Ok((row - 1, column - 1))
}

fn format_cell_range(range: CellRange) -> String {
    format!("{}:{}", format_cell(range.start), format_cell(range.end))
}

fn format_cell((row, column): (u32, u32)) -> String {
    let mut value = column + 1;
    let mut letters = Vec::new();
    while value > 0 {
        let remainder = (value - 1) % 26;
        letters.push((b'A' + remainder as u8) as char);
        value = (value - 1) / 26;
    }
    letters.reverse();
    format!(
        "{}{row_number}",
        letters.into_iter().collect::<String>(),
        row_number = row + 1
    )
}

fn excel_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::Import(error.to_string())
}

fn io_import_error(error: std::io::Error) -> CoreError {
    CoreError::Import(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Cell;

    #[test]
    fn parses_and_formats_excel_ranges() {
        assert_eq!(
            parse_cell_range("$B$3:AA20").unwrap(),
            CellRange {
                start: (2, 1),
                end: (19, 26)
            }
        );
        assert_eq!(
            format_cell_range(CellRange {
                start: (2, 1),
                end: (19, 26)
            }),
            "B3:AA20"
        );
        assert!(parse_cell_range("B8:A1").is_err());
        assert!(parse_cell_range("A0:B2").is_err());
    }

    #[test]
    fn preview_preserves_the_selected_rectangle_and_disambiguates_headers() {
        let values = Range::from_sparse(vec![
            Cell::new((3, 2), Data::String("Amount".into())),
            Cell::new((3, 3), Data::String("Amount".into())),
            Cell::new((4, 2), Data::Int(12)),
            Cell::new((4, 3), Data::String("West".into())),
        ]);
        let formulas = Range::<String>::empty();
        let preview = preview_from_ranges(
            &values,
            &formulas,
            CellRange {
                start: (3, 2),
                end: (4, 3),
            },
            true,
            20,
        )
        .unwrap();
        assert_eq!(preview.columns, ["Amount", "Amount 2"]);
        assert_eq!(preview.rows, [["12", "West"]]);
        assert_eq!(preview.row_count, 1);
    }

    #[test]
    fn a_defined_table_selection_restores_the_header_row() {
        assert_eq!(
            table_range_with_header((7, 2), (19, 5)),
            CellRange {
                start: (6, 2),
                end: (19, 5),
            }
        );
    }
}
