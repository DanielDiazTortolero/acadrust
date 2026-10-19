//! A pre-2018 multiline attribute keeps the tag it was stored with. Those releases down-save a multiline attribute as
//! one-line text plus an ACAD_MLATT record, and the outer tag carries an artificial suffix ("NOTE_001"; AutoCAD's own
//! DXF of such a drawing shows it). The reader resolves the real tag from the record ("NOTE"); the writers then wrote
//! that resolved tag back, so a save changed the stored tag of every such attribute (MILI's CB-0011). Now the stored
//! tag is written back to a pre-2018 file while the tag is unchanged; a retag, or a 2018 file, writes the tag.
use std::io::Cursor;

use acadrust::entities::{AttributeDefinition, EntityType};
use acadrust::objects::{XRecordEntry, XRecordValue};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfVersion, DxfWriter, Handle};

fn legacy_definition(version: DxfVersion) -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(version);
    let handle = doc
        .add_entity(EntityType::AttributeDefinition(AttributeDefinition::simple("NOTE_001")))
        .unwrap();
    doc.ensure_xrecord(handle, "ACAD_MLATT");
    doc.xrecord_mut(handle, "ACAD_MLATT").unwrap().entries = vec![
        XRecordEntry { code: 70, value: XRecordValue::Int16(4) },
        XRecordEntry { code: 2, value: XRecordValue::String("NOTE".into()) },
        XRecordEntry { code: 1, value: XRecordValue::String("Embedded Object".into()) },
    ];
    (doc, handle)
}

fn reread(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap())).read().unwrap()
}

fn tag_of(doc: &CadDocument, handle: Handle) -> String {
    match doc.get_entity(handle).unwrap() {
        EntityType::AttributeDefinition(d) => d.tag.clone(),
        _ => panic!("not an attribute definition"),
    }
}

/// The ATTDEF's group-2 tag in a DXF's text.
fn dxf_tag(doc: &CadDocument) -> String {
    let text = String::from_utf8(DxfWriter::new(doc).write_to_vec().unwrap()).unwrap();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let at = lines.iter().position(|l| *l == "AcDbAttributeDefinition").expect("an ATTDEF");
    let rest = &lines[at..];
    rest.windows(2).find(|w| w[0] == "2").map(|w| w[1].to_string()).expect("its tag")
}

#[test]
fn the_stored_tag_survives_two_saves_of_a_pre_2018_dwg() {
    for version in [DxfVersion::AC1018, DxfVersion::AC1021, DxfVersion::AC1024, DxfVersion::AC1027] {
        let (doc, handle) = legacy_definition(version);
        let once = reread(&doc);
        assert_eq!(tag_of(&once, handle), "NOTE", "{version:?}: the real tag, from the record");
        let twice = reread(&once);
        assert_eq!(tag_of(&twice, handle), "NOTE");
        assert_eq!(
            twice.legacy_attribute_tags.get(&handle).map(|(stored, _)| stored.as_str()),
            Some("NOTE_001"),
            "{version:?}: the second save wrote the resolved tag over the stored one"
        );
        assert_eq!(dxf_tag(&twice), "NOTE_001", "{version:?}: a pre-2018 DXF keeps the stored tag");
    }
}

#[test]
fn a_retag_or_a_2018_file_writes_the_tag() {
    let (doc, handle) = legacy_definition(DxfVersion::AC1021);
    let mut once = reread(&doc);
    if let Some(EntityType::AttributeDefinition(d)) = once.get_entity_mut(handle) {
        d.tag = "OTHER".into();
    }
    assert_eq!(dxf_tag(&once), "OTHER");
    let (doc, _) = legacy_definition(DxfVersion::AC1021);
    let mut once = reread(&doc);
    once.version = DxfVersion::AC1032;
    assert_eq!(dxf_tag(&once), "NOTE");
}
