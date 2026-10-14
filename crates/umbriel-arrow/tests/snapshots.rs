use polars_arrow::{
    array::{BooleanArray, PrimitiveArray, Utf8ViewArray},
    datatypes::ArrowDataType,
};
use umbriel_arrow::{Family, Replacement, SnapshotBatch, validate_batch, windows, workspaces};
use umbriel_protocol::{
    codec::decode,
    model::{Window, Workspace},
};

fn check_rows<T: serde::Serialize>(snapshot: &SnapshotBatch<()>, rows: &[T]) {
    validate_batch(snapshot.batch()).unwrap();
    assert_eq!(snapshot.batch().height(), rows.len());
    // Compare every wire field to its semantic column value, independently of
    // the schema declaration and emission implementation.
    for (row_index, row) in rows.iter().enumerate() {
        let object = sonic_rs::to_value(row).unwrap();
        let object = object.as_object().unwrap();
        assert_eq!(object.len(), snapshot.batch().width());
        for ((name, field), array) in snapshot
            .batch()
            .schema()
            .iter()
            .zip(snapshot.batch().arrays())
        {
            assert_eq!(field.dtype(), array.dtype());
            let expected = object.get(&name.as_str()).unwrap();
            match array.dtype() {
                ArrowDataType::Utf8View => assert_eq!(
                    array
                        .as_any()
                        .downcast_ref::<Utf8ViewArray>()
                        .unwrap()
                        .value(row_index),
                    expected.as_str().unwrap()
                ),
                ArrowDataType::Int32 => assert_eq!(
                    i64::from(
                        array
                            .as_any()
                            .downcast_ref::<PrimitiveArray<i32>>()
                            .unwrap()
                            .value(row_index)
                    ),
                    expected.as_i64().unwrap()
                ),
                ArrowDataType::Boolean => assert_eq!(
                    array
                        .as_any()
                        .downcast_ref::<BooleanArray>()
                        .unwrap()
                        .value(row_index),
                    expected.as_bool().unwrap()
                ),
                other => panic!("unexpected snapshot dtype: {other:?}"),
            }
        }
    }
}

use sonic_rs::{JsonContainerTrait, JsonValueTrait};

#[test]
fn full_wire_fields_survive_in_owned_columns() {
    let mut bytes =
        include_bytes!("../../umbriel-protocol/tests/fixtures/models/windows.json").to_vec();
    let mut rows: Vec<Window> = decode(&bytes).unwrap();
    rows[0].title = "long owned 雪🦦 text ".repeat(1000);
    rows[0].w = -13;
    rows[0].h = -14;
    rows[0].pid = i32::MIN;
    let batch = windows(&rows, ()).unwrap();
    check_rows(&batch, &rows);
    let title = rows[0].title.clone();
    rows[0].title.clear();
    drop(rows);
    bytes.fill(b'!');
    drop(bytes);
    let title_array = batch.batch().arrays()[9]
        .as_any()
        .downcast_ref::<Utf8ViewArray>()
        .unwrap();
    assert_eq!(title_array.value(0), title);

    let mut rows: Vec<Workspace> = decode(include_bytes!(
        "../../umbriel-protocol/tests/fixtures/models/workspaces.json"
    ))
    .unwrap();
    rows[0].index = -1;
    rows[0].name.clear();
    let batch = workspaces(&rows, ()).unwrap();
    check_rows(&batch, &rows);
    drop(rows);
    assert_eq!(batch.batch().height(), 2);
}

#[test]
fn empty_replacements_retain_schema_and_metadata() {
    let windows = windows(&[], (7_u64, 42_u64, "local observation time")).unwrap();
    let workspaces = workspaces(&[], (7_u64, 43_u64, "local observation time")).unwrap();
    for (snapshot, family, width, position) in [
        (windows, Family::Windows, 17, 42),
        (workspaces, Family::Workspaces, 9, 43),
    ] {
        validate_batch(snapshot.batch()).unwrap();
        assert!(snapshot.batch().is_empty());
        assert_eq!(snapshot.batch().width(), width);
        assert_eq!(snapshot.metadata().family, family);
        assert_eq!(snapshot.metadata().replacement, Replacement::Complete);
        assert_eq!(snapshot.metadata().schema_version, 1);
        assert_eq!(snapshot.metadata().provenance.1, position);
    }
}
