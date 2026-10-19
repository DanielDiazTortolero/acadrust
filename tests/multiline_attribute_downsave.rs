//! A multiline attribute keeps its text when a 2018 drawing is saved to an older release. A 2018 DWG keeps a
//! multiline value in the attribute's embedded MTEXT and leaves its one-line part empty; releases before 2018 have no
//! embedded MTEXT, and AutoCAD's own down-save puts the text in the one-line value (its 2007-2013 samples). This crate
//! wrote the stored one-line part, empty, so AutoCAD's sample saved as DWG 2013 read back with the attribute empty:
//! acadrust and LibreDWG both saw it (MILI's sweep of public drawings, 2026-10-19). Now an older release gets the text.
use std::io::Cursor;

use acadrust::entities::{AttributeEntity, EntityType, Insert};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Handle, Vector3};

const TEXT: &str = "my multi line text for the attrrib";

/// A 2018 drawing with a multiline attribute as a read of AutoCAD's 2018 sample leaves it: the text in the attribute,
/// and its stored one-line part empty (`legacy_attribute_values`), as AutoCAD writes it in 2018.
fn drawing_2018() -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut insert = Insert::new("*Model_Space", Vector3::new(10.0, 20.0, 0.0));
    let mut a = AttributeEntity::simple("MULTI_LINE_ATT", TEXT);
    a.is_multiline = true;
    let h = doc.allocate_handle();
    a.common.handle = h;
    insert.attributes.push(a);
    doc.add_entity(EntityType::Insert(insert)).unwrap();
    doc.legacy_attribute_values.insert(h, (String::new(), TEXT.to_string()));
    (doc, h)
}

/// The drawing's only attribute (found as it is read back, whatever handle it has then).
fn attribute(doc: &CadDocument, _h: Handle) -> AttributeEntity {
    doc.entities()
        .find_map(|e| match e {
            EntityType::Insert(i) => i.attributes.first().cloned(),
            _ => None,
        })
        .expect("the attribute")
}

#[test]
fn an_older_dwg_keeps_the_text() {
    let (doc, h) = drawing_2018();
    for version in [DxfVersion::AC1027, DxfVersion::AC1024, DxfVersion::AC1021, DxfVersion::AC1018] {
        let mut older = doc.clone();
        older.version = version;
        let back = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&older).unwrap())).read().unwrap();
        assert_eq!(attribute(&back, h).value, TEXT, "{version:?}: the attribute's text");
    }
}

#[test]
fn an_older_dxf_keeps_the_text() {
    let (doc, h) = drawing_2018();
    let mut older = doc.clone();
    older.version = DxfVersion::AC1027;
    let text = DxfWriter::new(&older).write_to_vec().unwrap();
    let back = DxfReader::from_reader(Cursor::new(text)).unwrap().read().unwrap();
    assert_eq!(attribute(&back, h).value, TEXT);
}

#[test]
fn a_2018_dwg_keeps_autocads_form() {
    // the one-line part stays as the drawing stored it, the text in the embedded MTEXT
    let (doc, h) = drawing_2018();
    let back = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap())).read().unwrap();
    let a = attribute(&back, h);
    assert_eq!(a.value, TEXT);
    assert!(a.is_multiline || a.embedded_mtext.is_some(), "no longer multiline");
}
