//! A layer that entities name but the DXF's LAYER table lacks is created on read, as AutoCAD's DXF import does, so a
//! DXF -> DWG conversion keeps the entities on their layer. Without the layer the DWG writer can only fall back to
//! layer "0" (an entity's layer is a required pointer, issue #80): ezdxf, for one, puts layout viewports on a
//! "VIEWPORTS" layer it never adds to the table, and those viewports moved to "0" (where their frames plot).
use std::io::Cursor;

use acadrust::entities::{EntityType, Line};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Vector3};

fn dxf_with_undefined_layer() -> (Vec<u8>, acadrust::Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut line = Line::from_points(Vector3::ZERO, Vector3::new(10.0, 0.0, 0.0));
    line.common.layer = "VIEWPORTS".into();
    let handle = doc.add_entity(EntityType::Line(line)).unwrap();
    let dxf = DxfWriter::new(&doc).write_to_vec().unwrap();
    let text = String::from_utf8_lossy(&dxf);
    let start = text.find("\nTABLES").unwrap();
    let tables = &text[start..start + text[start..].find("\nENDSEC").unwrap()];
    assert!(!tables.contains("VIEWPORTS"), "precondition: the layer appears only on the entity");
    (dxf, handle)
}

#[test]
fn an_undefined_layer_is_created_when_the_dxf_is_read() {
    let (dxf, _) = dxf_with_undefined_layer();
    let doc = DxfReader::from_reader(Cursor::new(dxf)).unwrap().read().unwrap();
    let layer = doc.layers.get("VIEWPORTS").expect("the layer was not created");
    assert!(!layer.handle.is_null());
}

#[test]
fn an_entity_on_an_undefined_layer_keeps_it_through_dwg() {
    let (dxf, handle) = dxf_with_undefined_layer();
    let doc = DxfReader::from_reader(Cursor::new(dxf)).unwrap().read().unwrap();
    let dwg = DwgWriter::write_to_vec(&doc).unwrap();
    let loaded = DwgReader::from_stream(Cursor::new(dwg)).read().unwrap();
    assert_eq!(loaded.get_entity(handle).unwrap().common().layer, "VIEWPORTS");
}
