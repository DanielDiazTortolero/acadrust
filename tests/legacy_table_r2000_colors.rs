//! Legacy ACAD_TABLE records keep the R2004 colour (CMC) encoding in an R2000 file: AutoCAD writes the
//! table class, newer than R2000, the R2004 way (as it does for TABLESTYLE). Reading those colours as R2000
//! colour indices misaligned every table record with a colour override.
//!
//! The AutoCAD-written check needs ACadSharp's public sample files (MIT): set ACADRUST_ACADSHARP_SAMPLES to
//! the folder holding sample_AC1015.dwg. Without it, only the synthetic round trip runs.
use std::io::Cursor;
use std::path::PathBuf;

use acadrust::entities::{CellStyle, EntityType, Table, TableCell};
use acadrust::types::{Color, DxfVersion, Handle, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter};

#[test]
fn colour_overrides_survive_an_r2000_round_trip() {
    let mut table = Table::new(Vector3::new(0.0, 0.0, 0.0), 1, 2);
    let mut cell = TableCell::text("A");
    let mut style = CellStyle::new();
    style.override_flags = 0x04 | 0x08 | 0x40;   // background, content and top border colours
    style.legacy_override_bits = true;
    style.background_color = Color::from_rgb(10, 20, 30);
    style.content_color = Color::from_index(1);
    style.top_border.color = Color::from_index(5);
    cell.style = Some(style);
    table.rows[0].cells[0] = cell;
    table.rows[0].cells[1] = TableCell::text("B");
    let mut doc = CadDocument::with_version(DxfVersion::AC1015);
    doc.add_entity(EntityType::Table(Box::new(table))).unwrap();
    let bytes = DwgWriter::write_to_vec(&doc).expect("DWG write");
    let read = DwgReader::from_stream(Cursor::new(bytes)).read().expect("DWG read");
    let t = read.entities().find_map(|e| match e {
        EntityType::Table(t) => Some((**t).clone()),
        _ => None,
    }).expect("table");
    let style = t.rows[0].cells[0].style.as_ref().expect("cell style");
    assert_eq!(style.background_color, Color::from_rgb(10, 20, 30));
    assert_eq!(style.content_color, Color::from_index(1));
    assert_eq!(style.top_border.color, Color::from_index(5));
    assert_eq!(t.rows[0].cells[1].text_value(), "B");
}

#[test]
fn autocad_r2000_table_reads_whole() {
    let Some(dir) = std::env::var_os("ACADRUST_ACADSHARP_SAMPLES") else {
        eprintln!("skipped: set ACADRUST_ACADSHARP_SAMPLES to ACadSharp's samples folder");
        return;
    };
    let path = PathBuf::from(dir).join("sample_AC1015.dwg");
    let doc = DwgReader::from_file(&path).expect("open").read().expect("an R2000 file with tables reads");
    let table = match doc.get_entity(Handle::from(0x528)) {
        Some(EntityType::Table(t)) => t.clone(),
        _ => panic!("table #528 missing"),
    };
    assert_eq!((table.rows.len(), table.columns.len()), (7, 3));
    let flags: Vec<i32> = table.rows.iter().flat_map(|r| r.cells.iter())
        .map(|c| c.style.as_ref().map_or(0, |s| s.override_flags)).collect();
    // AutoCAD's own values for this table (its R2004 save of the same drawing reads the same).
    assert_eq!(flags[12], 0x6666F, "the cell with colour and border overrides");
    assert!(flags.iter().all(|f| f & !0x7FFFF == 0), "no stray high bits: {flags:x?}");
    assert_eq!(table.rows[4].cells[0].text_value(), "This table will be imported as an           Insert entity");
    assert_eq!(table.rows[6].cells[2].text_value(), "cell text");
}
