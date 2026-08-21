//! Writing materialized values to delivery formats.

use crate::*;
use polars::prelude as pl;
use polars::prelude::{IntoLazy, SerWriter};
use std::collections::HashSet;
use std::fs;
use std::io::{Seek, Write};
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

enum ExcelCell {
    Blank,
    Number(f64),
    Boolean(bool),
    Text(String),
}

struct ExcelSheet {
    name: String,
    rows: Vec<Vec<ExcelCell>>,
}

impl Document {
    /// Write `frame_id`'s materialized values to `path` as CSV with one
    /// header row of column names. Values stay raw: ISO `YYYY-MM-DD` dates
    /// and plain numbers without currency or percentage decoration.
    pub(crate) fn export_frame_csv(&self, frame_id: &str, path: &Path) -> Result<(), CoreError> {
        let frame = self.frame(frame_id)?;
        let data_frame = self
            .materialize_frame_frame(frame_id, Layer::Data, &mut HashSet::new())
            .map_err(CoreError::Export)?;
        let mut data_frame = data_frame
            .lazy()
            .select(
                frame
                    .columns
                    .iter()
                    .map(|column| pl::col(column.id.clone()).alias(column.name.clone()))
                    .collect::<Vec<_>>(),
            )
            .collect()
            .map_err(|error| CoreError::Export(error.to_string()))?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| CoreError::Export(error.to_string()))?;
        }
        let file = fs::File::create(path).map_err(|error| CoreError::Export(error.to_string()))?;
        pl::CsvWriter::new(file)
            .include_header(true)
            .finish(&mut data_frame)
            .map_err(|error| CoreError::Export(error.to_string()))
    }

    /// Write a results-only workbook. Each selected frame is a worksheet and
    /// scalar objects share one compact Name/Value worksheet. Expressions are
    /// intentionally evaluated here rather than translated into Excel's
    /// formula language: the workbook is a handoff, not a second calculator.
    pub(crate) fn export_excel(&self, frame_ids: &[Id], path: &Path) -> Result<(), CoreError> {
        let mut sheets = Vec::new();
        let scalar_rows = self.excel_scalar_rows();
        if !scalar_rows.is_empty() {
            let mut rows = vec![vec![
                ExcelCell::Text("Name".into()),
                ExcelCell::Text("Value".into()),
            ]];
            rows.extend(scalar_rows);
            sheets.push(ExcelSheet {
                name: "Values".into(),
                rows,
            });
        }

        for frame_id in frame_ids {
            let frame = self.frame(frame_id)?;
            let data_frame = self
                .materialize_frame_frame(frame_id, Layer::Data, &mut HashSet::new())
                .map_err(CoreError::Export)?;
            let mut rows = vec![
                frame
                    .columns
                    .iter()
                    .map(|column| ExcelCell::Text(column.name.clone()))
                    .collect(),
            ];
            for row_index in 0..data_frame.height() {
                rows.push(
                    frame
                        .columns
                        .iter()
                        .map(|column| {
                            let series = data_frame
                                .column(&column.id)
                                .map_err(|error| CoreError::Export(error.to_string()))?;
                            let value = polars_value_at(series.as_materialized_series(), row_index)
                                .map_err(CoreError::Export)?;
                            Ok(scalar_excel_cell(value))
                        })
                        .collect::<Result<Vec<_>, CoreError>>()?,
                );
            }
            sheets.push(ExcelSheet {
                name: frame.name.clone(),
                rows,
            });
        }

        if sheets.is_empty() {
            return Err(CoreError::Export(
                "Select at least one table or add a named value to export".into(),
            ));
        }
        make_sheet_names_unique(&mut sheets);
        write_xlsx(path, &sheets)
    }

    fn excel_scalar_rows(&self) -> Vec<Vec<ExcelCell>> {
        let results = self.compute_results();
        let blocks = self.compute_blocks();
        let mut records = Vec::new();
        for object in &self.objects {
            match object {
                DataObject::Value(value) => records.push((
                    self.qualified_export_name(&value.id, &value.name),
                    value.id.as_str(),
                    scalar_excel_cell(
                        parse_scalar_value(&value.raw, value.data_type)
                            .unwrap_or_else(|_| ScalarValue::String(value.raw.clone())),
                    ),
                )),
                DataObject::Result(result) => {
                    let computed = &results[&result.id];
                    records.push((
                        self.qualified_export_name(&result.id, &result.name),
                        result.id.as_str(),
                        computed_excel_cell(&computed.cell),
                    ));
                }
                DataObject::Block(block) => {
                    let computed = &blocks[&block.id];
                    for line in &computed.lines {
                        if line.blank || line.comment {
                            continue;
                        }
                        records.push((
                            format!("{}.{}", block.name, line.name),
                            line.id.as_str(),
                            computed_excel_cell(&line.cell),
                        ));
                    }
                }
                _ => {}
            }
        }

        // Human paths resolve ordinary collisions. Documents can still carry
        // duplicate names at one level, so only those exceptional records get
        // a short stable identity suffix instead of producing ambiguous keys.
        let mut counts = std::collections::HashMap::new();
        for (name, _, _) in &records {
            *counts.entry(name.clone()).or_insert(0usize) += 1;
        }
        records
            .into_iter()
            .map(|(name, id, value)| {
                let name = if counts[&name] > 1 {
                    format!("{name} [{}]", id.chars().take(8).collect::<String>())
                } else {
                    name
                };
                vec![ExcelCell::Text(name), value]
            })
            .collect()
    }

    fn qualified_export_name(&self, object_id: &str, name: &str) -> String {
        let mut path = vec![name.to_string()];
        let mut current = object_id;
        while let Some(container) = self.container_of(current) {
            path.push(container.name.clone());
            current = &container.id;
        }
        path.reverse();
        path.join(".")
    }
}

fn computed_excel_cell(cell: &ComputedCell) -> ExcelCell {
    if cell.error.is_some() {
        ExcelCell::Text(cell.error.clone().unwrap_or_default())
    } else {
        scalar_excel_cell(cell.typed_value.clone())
    }
}

fn scalar_excel_cell(value: ScalarValue) -> ExcelCell {
    match value {
        ScalarValue::Null => ExcelCell::Blank,
        ScalarValue::Number(value) => ExcelCell::Number(value),
        ScalarValue::String(value) => ExcelCell::Text(value),
        ScalarValue::Boolean(value) => ExcelCell::Boolean(value),
        ScalarValue::Date(value) => ExcelCell::Text(value.to_string()),
    }
}

fn make_sheet_names_unique(sheets: &mut [ExcelSheet]) {
    let mut used = HashSet::new();
    for sheet in sheets {
        let base = sanitized_sheet_name(&sheet.name);
        let mut candidate = base.clone();
        let mut suffix = 2;
        while used.contains(&candidate.to_lowercase()) {
            let tail = format!(" ({suffix})");
            candidate = format!(
                "{}{}",
                base.chars()
                    .take(31usize.saturating_sub(tail.len()))
                    .collect::<String>(),
                tail
            );
            suffix += 1;
        }
        used.insert(candidate.to_lowercase());
        sheet.name = candidate;
    }
}

fn sanitized_sheet_name(name: &str) -> String {
    let cleaned = name
        .chars()
        .filter(|character| !matches!(character, '[' | ']' | ':' | '*' | '?' | '/' | '\\'))
        .take(31)
        .collect::<String>();
    let cleaned = cleaned.trim_matches('\'').trim();
    if cleaned.is_empty() {
        "Sheet".into()
    } else {
        cleaned.into()
    }
}

fn write_xlsx(path: &Path, sheets: &[ExcelSheet]) -> Result<(), CoreError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| CoreError::Export(error.to_string()))?;
    }
    let file = fs::File::create(path).map_err(|error| CoreError::Export(error.to_string()))?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip_entry(
        &mut archive,
        "[Content_Types].xml",
        &content_types_xml(sheets.len()),
        options,
    )?;
    zip_entry(&mut archive, "_rels/.rels", ROOT_RELS, options)?;
    zip_entry(
        &mut archive,
        "xl/workbook.xml",
        &workbook_xml(sheets),
        options,
    )?;
    zip_entry(
        &mut archive,
        "xl/_rels/workbook.xml.rels",
        &workbook_rels_xml(sheets.len()),
        options,
    )?;
    for (index, sheet) in sheets.iter().enumerate() {
        zip_entry(
            &mut archive,
            &format!("xl/worksheets/sheet{}.xml", index + 1),
            &worksheet_xml(sheet),
            options,
        )?;
    }
    archive
        .finish()
        .map_err(|error| CoreError::Export(error.to_string()))?;
    Ok(())
}

fn zip_entry<W: Write + Seek>(
    archive: &mut ZipWriter<W>,
    name: &str,
    contents: &str,
    options: SimpleFileOptions,
) -> Result<(), CoreError> {
    archive
        .start_file(name, options)
        .map_err(|error| CoreError::Export(error.to_string()))?;
    archive
        .write_all(contents.as_bytes())
        .map_err(|error| CoreError::Export(error.to_string()))
}

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;

fn content_types_xml(count: usize) -> String {
    let sheets = (1..=count).map(|index| format!(r#"<Override PartName="/xl/worksheets/sheet{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#)).collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>{sheets}</Types>"#
    )
}

fn workbook_xml(sheets: &[ExcelSheet]) -> String {
    let sheets = sheets
        .iter()
        .enumerate()
        .map(|(index, sheet)| {
            format!(
                r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
                xml_escape(&sheet.name),
                index + 1,
                index + 1
            )
        })
        .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>{sheets}</sheets></workbook>"#
    )
}

fn workbook_rels_xml(count: usize) -> String {
    let relationships = (1..=count).map(|index| format!(r#"<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.xml"/>"#)).collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{relationships}</Relationships>"#
    )
}

fn worksheet_xml(sheet: &ExcelSheet) -> String {
    let rows = sheet.rows.iter().enumerate().map(|(row_index, row)| {
        let cells = row.iter().enumerate().filter_map(|(column_index, cell)| {
            let reference = format!("{}{}", excel_column_name(column_index), row_index + 1);
            match cell {
                ExcelCell::Blank => None,
                ExcelCell::Number(value) => Some(format!(r#"<c r="{reference}"><v>{value}</v></c>"#)),
                ExcelCell::Boolean(value) => Some(format!(r#"<c r="{reference}" t="b"><v>{}</v></c>"#, usize::from(*value))),
                ExcelCell::Text(value) => Some(format!(r#"<c r="{reference}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#, xml_escape(value))),
            }
        }).collect::<String>();
        format!(r#"<row r="{}">{cells}</row>"#, row_index + 1)
    }).collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{rows}</sheetData></worksheet>"#
    )
}

fn excel_column_name(mut index: usize) -> String {
    let mut name = String::new();
    loop {
        name.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    name
}

fn xml_escape(value: &str) -> String {
    value
        .chars()
        .filter(|character| matches!(*character, '\t' | '\n' | '\r') || *character >= ' ')
        .fold(String::new(), |mut escaped, character| {
            match character {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&apos;"),
                _ => escaped.push(character),
            }
            escaped
        })
}
