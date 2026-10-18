//! R2000 DXF spells the data table record and its subclass marker ACDBDATATABLE: AutoCAD writes
//! "0 ACDBDATATABLE / 100 ACDBDATATABLE" where R2004+ writes "0 DATATABLE / 100 AcDbDataTable". The reader
//! looked its fields up under the exact R2004 marker, so an R2000 data table read back empty (and was then
//! written to DWG as an empty record AutoCAD rejects).
use std::io::Cursor;

use acadrust::objects::{ClassObject, ClassObjectData, DataTable, ObjectType};
use acadrust::types::Vector3;
use acadrust::DxfReader;

/// The native AC1032 fixture with the data table's record type and subclass marker respelled the R2000 way
/// (only group 0 and group 100 values change).
fn r2000_spelled() -> Vec<u8> {
    let text = String::from_utf8_lossy(include_bytes!("datatable/point_object_id.dxf")).into_owned();
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split(eol).collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i + 1 < lines.len() {
        let (code, value) = (lines[i], lines[i + 1]);
        let renamed = match (code.trim(), value.trim()) {
            ("0", "DATATABLE") | ("100", "AcDbDataTable") => "ACDBDATATABLE",
            _ => value,
        };
        out.push(code.to_string());
        out.push(renamed.to_string());
        i += 2;
    }
    out.extend(lines[i..].iter().map(|s| s.to_string()));
    out.join(eol).into_bytes()
}

fn data_table(bytes: Vec<u8>) -> DataTable {
    let doc = DxfReader::from_reader(Cursor::new(bytes)).unwrap().read().unwrap();
    doc.objects
        .values()
        .find_map(|object| match object {
            ObjectType::ClassObject(ClassObject { data: ClassObjectData::DataTable(t), .. }) => Some(t.clone()),
            _ => None,
        })
        .expect("the data table stays a native object")
}

#[test]
fn an_r2000_spelled_data_table_keeps_its_rows_and_columns() {
    let native = data_table(include_bytes!("datatable/point_object_id.dxf").to_vec());
    let r2000 = data_table(r2000_spelled());
    assert_eq!(native.columns.len(), 2, "the fixture itself");
    assert_eq!(r2000.row_count, native.row_count);
    assert_eq!(r2000.columns.len(), native.columns.len());
    assert_eq!(r2000.columns[0].rows[0].point, Vector3::new(1.5, 2.5, 3.5));
    assert_eq!(r2000, native);
}
