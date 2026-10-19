//! An untouched INSERT with attributes is copied verbatim with its ATTRIBs and SEQEND, like every other untouched
//! entity. The writer re-encoded every INSERT that owns attributes from its own reading (only childless records were
//! copied), so any imprecision in that reading reached every AutoCAD Electrical component of a patched drawing
//! (MILI's CB-0006 and CB-0011 did). The records still carry the bytes they were read from: an INSERT, its ATTRIBs and
//! its SEQEND that the edit didn't touch keep them; a touched one is encoded from the model as before.
use std::io::Cursor;
use std::sync::Arc;

use acadrust::entities::{AttributeEntity, EntityType, Insert, RawRecord};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfVersion, Handle, Vector3};

/// A drawing with one INSERT owning two attributes (TAG1, DESC1), the second one valued `desc`.
fn drawing(desc: &str) -> (CadDocument, Handle) {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut insert = Insert::new("*Model_Space", Vector3::new(10.0, 20.0, 0.0));
    insert.attributes.push(AttributeEntity::simple("TAG1", "PB1"));
    insert.attributes.push(AttributeEntity::simple("DESC1", desc));
    let handle = doc.add_entity(EntityType::Insert(insert)).unwrap();
    (doc, handle)
}

fn reread(doc: &CadDocument) -> CadDocument {
    DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(doc).unwrap())).read().unwrap()
}

fn insert(doc: &CadDocument, h: Handle) -> &Insert {
    match doc.get_entity(h).unwrap() {
        EntityType::Insert(i) => i,
        _ => panic!("not an insert"),
    }
}

type Raws = (Option<Arc<RawRecord>>, Vec<Option<Arc<RawRecord>>>);

fn raws(doc: &CadDocument, h: Handle) -> Raws {
    let i = insert(doc, h);
    (i.common.raw_record.clone(), i.attributes.iter().map(|a| a.common.raw_record.clone()).collect())
}

/// `doc` as it was read, but its INSERT and ATTRIB records carrying the bytes of `from` (the same handles): what an
/// untouched component looks like to the writer when its model and its bytes could disagree.
fn with_bytes_of(mut doc: CadDocument, h: Handle, from: Raws) -> CadDocument {
    if let Some(EntityType::Insert(i)) = doc.get_entity_mut(h) {
        i.common.raw_record = from.0;
        for (a, raw) in i.attributes.iter_mut().zip(from.1) {
            a.common.raw_record = raw;
        }
    }
    doc
}

#[test]
fn the_records_carry_their_bytes() {
    let (doc, h) = drawing("PUMP");
    let read = reread(&doc);
    let (ins, atts) = raws(&read, h);
    assert!(ins.is_some(), "the INSERT keeps its bytes");
    assert!(atts.len() == 2 && atts.iter().all(Option::is_some), "each ATTRIB keeps its bytes");
}

#[test]
fn an_untouched_component_is_copied_not_re_encoded() {
    let (a, h) = drawing("PUMP");
    let (b, hb) = drawing("FAN");
    assert_eq!(h, hb);
    let bytes_of_a = raws(&reread(&a), h);
    let out = reread(&with_bytes_of(reread(&b), h, bytes_of_a));
    let i = insert(&out, h);
    assert_eq!(i.attributes[1].value, "PUMP", "the ATTRIB was re-encoded from the model, not copied");
    assert_eq!(i.attributes.len(), 2);
    assert!(i.seqend_handle.is_some(), "the SEQEND is still there");
}

#[test]
fn a_component_copied_twice_keeps_every_byte() {
    let (doc, h) = drawing("PUMP");
    let once = reread(&doc);
    let twice = reread(&once);
    assert_eq!(raws(&once, h), raws(&twice, h), "a second save changed the component's records");
}

#[test]
fn a_touched_component_is_encoded_from_the_model() {
    let (doc, h) = drawing("PUMP");
    let mut read = reread(&doc);
    if let Some(EntityType::Insert(i)) = read.get_entity_mut(h) {
        i.attributes[1].value = "FAN".into();
    }
    assert_eq!(insert(&reread(&read), h).attributes[1].value, "FAN");
}
