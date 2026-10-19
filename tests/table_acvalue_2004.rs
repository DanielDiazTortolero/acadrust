//! A table's cell values carry an ACVALUE_END marker in DXF only from R2007 on, as AutoCAD writes them (its own
//! sample DXFs: 110 markers in every release from 2007, none in 2000 and 2004). This crate wrote the marker after a
//! modern cell's value and after each custom-data value in every release, and AutoCAD's DXF reader discarded an
//! R2004 drawing at the first table ("Premature end of object" in TABLECONTENT: DWG TrueView 2027 on AutoCAD's own
//! sample read and written here, 2026-10-19; MILI's CB-0043).
use acadrust::entities::table::{CellContent, CellValue, TableCustomData};
use acadrust::entities::Table;
use acadrust::objects::ObjectType;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DxfWriter};

/// A DXF of a drawing whose TABLECONTENT has one cell, valued "Table sample", with a custom-data item: the modern
/// table form a drawing read from AutoCAD's DWG has.
fn dxf(version: DxfVersion) -> String {
    let mut doc = CadDocument::with_version(version);
    let mut content = Table::new(Vector3::ZERO, 1, 1);
    let cell = &mut content.rows[0].cells[0];
    cell.contents.push(CellContent::text("Table sample"));
    cell.custom_data_items.push(TableCustomData { name: "MILI".into(), value: CellValue::text("1") });
    let h = doc.allocate_handle();
    content.common.handle = h;
    doc.objects.insert(h, ObjectType::TableContent(content));
    String::from_utf8_lossy(&DxfWriter::new(&doc).write_to_vec().unwrap()).into_owned()
}

fn count(text: &str, line: &str) -> usize {
    text.lines().filter(|l| l.trim() == line).count()
}

#[test]
fn only_from_2007_on() {
    let r2007 = dxf(DxfVersion::AC1021);
    assert!(count(&r2007, "LINKEDTABLEDATACELL_BEGIN") > 0, "no modern cell written: the test doesn't reach it");
    assert_eq!(count(&r2007, "ACVALUE_END"), 2, "the cell's value and its custom data, each closed in 2007");
    assert_eq!(count(&dxf(DxfVersion::AC1032), "ACVALUE_END"), 2);
    let r2004 = dxf(DxfVersion::AC1018);
    assert!(count(&r2004, "LINKEDTABLEDATACELL_BEGIN") > 0);
    assert_eq!(count(&r2004, "ACVALUE_END"), 0, "a 2004 DXF");
}
