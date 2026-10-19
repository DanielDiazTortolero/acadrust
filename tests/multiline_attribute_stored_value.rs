//! A multiline attribute keeps the one-line value it was stored with. AutoCAD stores a multiline attribute's text in
//! its embedded MTEXT and leaves the one-line part empty (its own sample DWGs and their DXF show it). The reader takes
//! the value from the MTEXT, and the writers put that whole text into the one-line part too, so every save filled in
//! the empty part of such attributes (MILI's CB-0006). Now the stored one-line value is written back while the value
//! is unchanged; after an edit, a drawing that stored it empty keeps it empty. A new attribute is written as before.
use std::io::Cursor;

use acadrust::entities::{AttributeEntity, EntityType, Insert, MText};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfVersion, DxfWriter, Handle, Vector3};

const TEXT: &str = "First\\PSecond";

/// A drawing with a multiline attribute stored the way AutoCAD stores it (one-line part `one_line`).
fn drawing(one_line: &str) -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut att = AttributeEntity::simple("TITLE", one_line);
    att.is_multiline = true;
    att.embedded_mtext = Some(Box::new(MText::with_value(TEXT, Vector3::ZERO)));
    let mut insert = Insert::new("*Model_Space", Vector3::ZERO);
    insert.attributes.push(att);
    let handle = doc.add_entity(EntityType::Insert(insert)).unwrap();
    (doc, handle)
}

fn reread(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap())).read().unwrap()
}

fn attribute(doc: &CadDocument, insert: Handle) -> &AttributeEntity {
    match doc.get_entity(insert).unwrap() {
        EntityType::Insert(i) => &i.attributes[0],
        _ => panic!("not an insert"),
    }
}

/// The ATTRIB's one-line value (its first group 1, before the embedded object) in a DXF's text.
fn dxf_one_line(doc: &CadDocument) -> String {
    let text = String::from_utf8(DxfWriter::new(doc).write_to_vec().unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let at = lines.windows(2).position(|w| w[0] == "0" && w[1] == "ATTRIB").expect("an ATTRIB");
    lines[at..].windows(2).find(|w| w[0] == "1").map(|w| w[1].to_string()).expect("its value")
}

#[test]
fn an_empty_one_line_part_survives_two_saves_and_a_dxf() {
    let (doc, insert) = drawing("");
    let once = reread(&doc);
    assert_eq!(attribute(&once, insert).value, TEXT, "the value comes from the embedded MTEXT");
    let handle = attribute(&once, insert).common.handle;
    let twice = reread(&once);
    assert_eq!(attribute(&twice, insert).value, TEXT);
    assert_eq!(
        twice.legacy_attribute_values.get(&handle).map(|(stored, _)| stored.as_str()),
        Some(""),
        "the second save filled in the empty one-line part"
    );
    assert_eq!(dxf_one_line(&twice), "", "the DXF keeps the one-line part empty, as AutoCAD writes it");
}

#[test]
fn after_an_edit_an_empty_one_line_part_stays_empty() {
    let (doc, insert) = drawing("");
    let mut once = reread(&doc);
    if let Some(EntityType::Insert(i)) = once.get_entity_mut(insert) {
        let a = &mut i.attributes[0];
        a.value = "New".into();
        a.embedded_mtext.as_mut().unwrap().value = "New".into();
    }
    assert_eq!(dxf_one_line(&once), "");
    let back = reread(&once);
    assert_eq!(attribute(&back, insert).value, "New");
}

#[test]
fn a_new_multiline_attribute_is_written_as_before() {
    let (doc, insert) = drawing(TEXT);
    assert_eq!(dxf_one_line(&doc), TEXT);
    let once = reread(&doc);
    assert_eq!(attribute(&once, insert).value, TEXT);
    assert!(once.legacy_attribute_values.is_empty(), "nothing to remember: the one-line part was the value");
}
