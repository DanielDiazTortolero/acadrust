//! A layer's stored `AcCmTransparency` value is written back as it was while the layer's transparency is unchanged.
//! AutoCAD writes 0 for an opaque layer (every layer of its own sample drawings has it); the reader decodes 0 as
//! opaque, and the writers re-encoded opaque as 33554687 ("by value, alpha 255"), so every layer of a drawing the
//! codec saved came back rewritten, also in an otherwise byte-faithful same-version DWG save. A changed transparency
//! is still written in the canonical form.
use std::io::Cursor;

use acadrust::tables::Layer;
use acadrust::types::{DxfVersion, Transparency};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfWriter};

/// A DXF whose layer RAW_ZERO stores AcCmTransparency 0, as AutoCAD writes it.
fn dxf_with_stored_zero() -> Vec<u8> {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut layer = Layer::new("RAW_ZERO");
    layer.transparency = Transparency::Explicit(1);
    document.layers.add(layer).unwrap();
    let bytes = DxfWriter::new(&document).write_to_vec().unwrap();
    let mut lines: Vec<String> = String::from_utf8(bytes).unwrap().lines().map(|l| l.trim().to_string()).collect();
    let at = lines
        .windows(4)
        .position(|w| w == ["1001", "AcCmTransparency", "1071", "33554686"])
        .expect("layer transparency XDATA");
    lines[at + 3] = "0".to_string();
    lines.join("\n").into_bytes()
}

/// The AcCmTransparency value in a layer's record (from its name to the next record) of a DXF's text.
fn value_of(dxf: &[u8], layer: &str) -> String {
    let lines: Vec<String> = String::from_utf8_lossy(dxf).lines().map(|l| l.trim().to_string()).collect();
    let at = lines.windows(2).position(|w| w[0] == "2" && w[1] == layer).expect("the layer's record");
    let rest = &lines[at..];
    let end = rest.iter().skip(1).position(|l| l == "LAYER" || l == "ENDTAB").unwrap_or(rest.len());
    rest[..end + 1]
        .windows(4)
        .find(|w| w[0] == "1001" && w[1] == "AcCmTransparency" && w[2] == "1071")
        .map(|w| w[3].clone())
        .expect("the layer's AcCmTransparency value")
}

#[test]
fn a_stored_zero_survives_a_dxf_round_trip() {
    let doc = DxfReader::from_reader(Cursor::new(dxf_with_stored_zero())).unwrap().read().unwrap();
    assert_eq!(doc.layers.get("RAW_ZERO").unwrap().transparency, Transparency::OPAQUE);
    let out = DxfWriter::new(&doc).write_to_vec().unwrap();
    assert_eq!(value_of(&out, "RAW_ZERO"), "0");
}

#[test]
fn a_stored_zero_survives_dwg_and_a_second_dwg_save() {
    let doc = DxfReader::from_reader(Cursor::new(dxf_with_stored_zero())).unwrap().read().unwrap();
    let dwg = DwgWriter::write_to_vec(&doc).unwrap();
    let once = DwgReader::from_stream(Cursor::new(dwg)).read().unwrap();
    assert_eq!(value_of(&DxfWriter::new(&once).write_to_vec().unwrap(), "RAW_ZERO"), "0");
    let dwg2 = DwgWriter::write_to_vec(&once).unwrap();
    let twice = DwgReader::from_stream(Cursor::new(dwg2)).read().unwrap();
    assert_eq!(twice.layers.get("RAW_ZERO").unwrap().transparency, Transparency::OPAQUE);
    assert_eq!(value_of(&DxfWriter::new(&twice).write_to_vec().unwrap(), "RAW_ZERO"), "0");
}

#[test]
fn a_changed_transparency_is_written_canonically() {
    let mut doc = DxfReader::from_reader(Cursor::new(dxf_with_stored_zero())).unwrap().read().unwrap();
    doc.layers.get_mut("RAW_ZERO").unwrap().transparency = Transparency::Explicit(64);
    let out = DxfWriter::new(&doc).write_to_vec().unwrap();
    assert_eq!(value_of(&out, "RAW_ZERO"), ((2u32 << 24) | (255 - 64)).to_string());
    let dwg = DwgWriter::write_to_vec(&doc).unwrap();
    let back = DwgReader::from_stream(Cursor::new(dwg)).read().unwrap();
    assert_eq!(back.layers.get("RAW_ZERO").unwrap().transparency, Transparency::Explicit(64));
}

#[test]
fn the_stored_value_is_kept_only_while_it_decodes_to_the_layer_transparency() {
    assert_eq!(Transparency::OPAQUE.to_layer_alpha_value(Some(0)), 0);
    assert_eq!(Transparency::OPAQUE.to_layer_alpha_value(Some(33554687)), 33554687);
    assert_eq!(Transparency::OPAQUE.to_layer_alpha_value(None), 33554687);
    assert_eq!(Transparency::Explicit(64).to_layer_alpha_value(Some(0)), ((2u32 << 24) | 191) as i32);
}
