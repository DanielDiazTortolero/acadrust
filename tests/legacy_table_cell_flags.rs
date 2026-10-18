//! Legacy (pre-R2010) ACAD_TABLE cell overrides follow the ODA layout: each border has its own colour,
//! line-weight and visibility bit (0x40/0x400/0x4000 top, 0x80/0x800/0x8000 right, 0x100/0x1000/0x10000
//! bottom, 0x200/0x2000/0x20000 left), and a visibility value is stored only when its visibility bit is set.
//! Gating it on the line-weight bit instead desynchronised every cell whose two bits differ (tables read from
//! DXF carry line weights without visibility), and AutoCAD then discarded the table and other objects.
use std::io::Cursor;

use acadrust::entities::table::cell_override_bit;
use acadrust::entities::{CellStyle, EntityType, Table, TableCell};
use acadrust::types::{DxfVersion, LineWeight, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter};

const TOP_WEIGHT: i32 = 0x400;
const TOP_VISIBILITY: i32 = 0x4000;

fn styled_cell(text: &str, flags: i32, invisible: bool) -> TableCell {
    let mut cell = TableCell::text(text);
    let mut style = CellStyle::new();
    style.override_flags = flags;
    style.legacy_override_bits = true;
    style.top_border.line_weight = LineWeight::from_value(50);
    style.top_border.invisible = invisible;
    cell.style = Some(style);
    cell
}

/// A one-row table whose first cell carries the given overrides; the second cell is plain text, so a
/// desynchronised first cell shows up as a lost second one.
fn roundtrip(version: DxfVersion, flags: i32, invisible: bool) -> Table {
    let mut table = Table::new(Vector3::new(0.0, 0.0, 0.0), 1, 2);
    table.rows[0].cells[0] = styled_cell("A", flags, invisible);
    table.rows[0].cells[1] = TableCell::text("B");
    let mut doc = CadDocument::with_version(version);
    doc.add_entity(EntityType::Table(Box::new(table))).unwrap();
    let bytes = DwgWriter::write_to_vec(&doc).expect("DWG write");
    let read = DwgReader::from_stream(Cursor::new(bytes)).read().expect("DWG read");
    let found = read.entities().find_map(|e| match e {
        EntityType::Table(t) => Some((**t).clone()),
        _ => None,
    });
    found.expect("table missing after the DWG round trip")
}

#[test]
fn visibility_codes_map_to_their_own_bits() {
    for (code, bit) in [(279, 0x400), (275, 0x800), (276, 0x1000), (278, 0x2000),
                        (289, 0x4000), (285, 0x8000), (286, 0x10000), (288, 0x20000)] {
        assert_eq!(cell_override_bit(code), Some(bit), "DXF code {code}");
    }
}

#[test]
fn a_line_weight_override_alone_stores_no_visibility() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1021] {
        let t = roundtrip(version, TOP_WEIGHT, true);
        let style = t.rows[0].cells[0].style.as_ref().expect("cell style");
        assert_eq!(style.override_flags & 0x3FFFF, TOP_WEIGHT, "{version:?}: flags");
        assert_eq!(style.top_border.line_weight, LineWeight::from_value(50), "{version:?}: line weight");
        assert!(!style.top_border.invisible, "{version:?}: visibility was not overridden, so it isn't stored");
        assert_eq!(t.rows[0].cells[1].text_value(), "B", "{version:?}: the next cell");
    }
}

#[test]
fn a_visibility_override_alone_keeps_its_value() {
    for version in [DxfVersion::AC1015, DxfVersion::AC1018, DxfVersion::AC1021] {
        let t = roundtrip(version, TOP_VISIBILITY, true);
        let style = t.rows[0].cells[0].style.as_ref().expect("cell style");
        assert_eq!(style.override_flags & 0x3FFFF, TOP_VISIBILITY, "{version:?}: flags");
        assert!(style.top_border.invisible, "{version:?}: the visibility override");
        assert_eq!(t.rows[0].cells[1].text_value(), "B", "{version:?}: the next cell");
    }
}
