//! Attributes of a block reference on a paper-space layout survive DXF -> DWG -> read.
//!
//! A DXF marks every paper-space entity with group 67 = 1, sub-entities (ATTRIB, SEQEND) included, and the DXF
//! reader keeps that as `entity_mode = 1`. The DWG writer used the stored mode as it was, so the attributes went out
//! as "an entity of the active paper space" without their owner handle; the DWG reader then filed them under the
//! *Paper_Space block, where no INSERT claims them, and they were lost (a title block on a layout lost every value).
//! In DWG a sub-entity, and anything on a layout other than the active one, carries entity mode 0 and its owner.
use std::io::Cursor;

use acadrust::entities::{AttributeEntity, EntityType, Insert};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Handle, Vector3};

/// A document with an INSERT carrying two attributes on `layout` ("" = model space).
fn title_block(layout: &str) -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.add_layout("E-101").unwrap();
    let mut insert = Insert::new("*Model_Space", Vector3::new(10.0, 5.0, 0.0));
    insert.attributes.push(AttributeEntity::simple("DWGNO", "E-101"));
    insert.attributes.push(AttributeEntity::simple("REV", "A"));
    let handle = if layout.is_empty() {
        doc.add_entity(EntityType::Insert(insert)).unwrap()
    } else {
        doc.add_entity_to_layout(EntityType::Insert(insert), layout).unwrap()
    };
    (doc, handle)
}

fn through_dxf(doc: &CadDocument) -> CadDocument {
    let dxf = DxfWriter::new(doc).write_to_vec().unwrap();
    DxfReader::from_reader(Cursor::new(dxf)).unwrap().read().unwrap()
}

fn attributes_after_dwg(doc: &CadDocument, handle: Handle) -> Vec<(String, String)> {
    let dwg = DwgWriter::write_to_vec(doc).unwrap();
    let loaded = DwgReader::from_stream(Cursor::new(dwg)).read().unwrap();
    let Some(EntityType::Insert(insert)) = loaded.get_entity(handle) else {
        panic!("the INSERT is gone")
    };
    insert.attributes.iter().map(|a| (a.tag.clone(), a.value.clone())).collect()
}

fn expected() -> Vec<(String, String)> {
    vec![("DWGNO".into(), "E-101".into()), ("REV".into(), "A".into())]
}

#[test]
fn attributes_on_a_second_layout_survive_dxf_to_dwg() {
    let (doc, handle) = title_block("E-101");
    assert_eq!(attributes_after_dwg(&through_dxf(&doc), handle), expected());
}

#[test]
fn attributes_on_the_active_layout_survive_dxf_to_dwg() {
    let (doc, handle) = title_block("Layout1");
    assert_eq!(attributes_after_dwg(&through_dxf(&doc), handle), expected());
}

#[test]
fn attributes_in_model_space_survive_dxf_to_dwg() {
    let (doc, handle) = title_block("");
    assert_eq!(attributes_after_dwg(&through_dxf(&doc), handle), expected());
}

#[test]
fn a_stored_paper_space_mode_on_an_attribute_does_not_detach_it() {
    // the same state the DXF reader leaves, set directly: the attribute is still its INSERT's child
    let (mut doc, handle) = title_block("E-101");
    if let Some(EntityType::Insert(insert)) = doc.get_entity_mut(handle) {
        insert.common.entity_mode = Some(1);
        for attribute in &mut insert.attributes {
            attribute.common.entity_mode = Some(1);
        }
    }
    assert_eq!(attributes_after_dwg(&doc, handle), expected());
}
