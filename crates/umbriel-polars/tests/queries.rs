use polars::prelude::{DataFrame, DataType, col, lit};
use umbriel_arrow::{Family, Replacement, validate_batch, windows, workspaces};
use umbriel_polars::{ResultKind, SnapshotFrame};
use umbriel_protocol::{
    codec::decode,
    model::{Window, Workspace},
};

fn rows() -> Vec<Window> {
    decode(include_bytes!(
        "../../umbriel-protocol/tests/fixtures/models/windows.json"
    ))
    .unwrap()
}

#[test]
fn finite_lazy_projection_filter_and_native_output() {
    let rows = rows();
    let input = windows(&rows, (3_u64, 19_u64, "local time")).unwrap();
    drop(rows);
    let frame = SnapshotFrame::from(input);
    assert_eq!(
        frame.frame().column("title").unwrap().dtype(),
        &DataType::String
    );
    let output = frame
        .lazy()
        .map(|plan| {
            plan.filter(col("pid").lt(lit(0_i32))).select([
                col("title"),
                col("x"),
                col("content_type"),
            ])
        })
        .collect()
        .unwrap();
    assert_eq!(output.kind(), ResultKind::Derived);
    assert_eq!(output.source().family, Family::Windows);
    assert_eq!(output.source().replacement, Replacement::Complete);
    assert_eq!(output.source().provenance, (3, 19, "local time"));
    assert_eq!(output.batches().len(), 1);
    let batch = &output.batches()[0];
    validate_batch(batch).unwrap();
    assert_eq!(batch.height(), 1);
    assert_eq!(
        batch
            .schema()
            .iter_names()
            .map(|n| n.as_str())
            .collect::<Vec<_>>(),
        ["title", "x", "content_type"]
    );
    // Only a batch generated from a valid Polars frame is converted here.
    let frame = DataFrame::from(batch.clone());
    assert_eq!(frame.column("x").unwrap().i32().unwrap().get(0), Some(-20));
    assert_eq!(
        frame.column("content_type").unwrap().str().unwrap().get(0),
        Some("future-content")
    );
    assert_eq!(
        frame.column("title").unwrap().str().unwrap().get(0),
        Some("Scratch 🦦")
    );
}

#[test]
fn empty_query_output_and_empty_input_keep_schema_and_provenance() {
    for rows in [rows(), vec![]] {
        let output = SnapshotFrame::from(windows(&rows, "observation").unwrap())
            .lazy()
            .map(|plan| plan.filter(lit(false)).select([col("id"), col("pid")]))
            .collect()
            .unwrap();
        assert_eq!(output.source().provenance, "observation");
        assert_eq!(output.kind(), ResultKind::Derived);
        assert_eq!(output.batches().len(), 1);
        let batch = &output.batches()[0];
        validate_batch(batch).unwrap();
        assert_eq!((batch.height(), batch.width()), (0, 2));
        let frame = DataFrame::from(batch.clone());
        assert_eq!(frame.column("id").unwrap().dtype(), &DataType::String);
    }
}

#[test]
fn workspaces_support_native_lazy_expressions_and_errors_are_returned() {
    let rows: Vec<Workspace> = decode(include_bytes!(
        "../../umbriel-protocol/tests/fixtures/models/workspaces.json"
    ))
    .unwrap();
    let input = workspaces(&rows, 7).unwrap();
    let output = SnapshotFrame::from(input.clone())
        .lazy()
        .map(|plan| {
            plan.filter(col("occupied"))
                .select([col("id"), col("layout")])
        })
        .collect()
        .unwrap();
    assert_eq!(output.source().family, Family::Workspaces);
    validate_batch(&output.batches()[0]).unwrap();
    assert_eq!(output.batches()[0].height(), 1);
    assert!(
        SnapshotFrame::from(input)
            .lazy()
            .map(|plan| plan.select([col("missing")]))
            .collect()
            .is_err()
    );
}

#[test]
fn aggregation_and_zero_column_results_are_not_family_replacements() {
    let input = windows(&rows(), ()).unwrap();
    let aggregate = SnapshotFrame::from(input.clone())
        .lazy()
        .map(|plan| plan.select([col("pid").min()]))
        .collect()
        .unwrap();
    assert_eq!(aggregate.kind(), ResultKind::Derived);
    assert_eq!(aggregate.batches()[0].height(), 1);
    let output = SnapshotFrame::from(input)
        .lazy()
        .map(|plan| plan.select(Vec::<polars::prelude::Expr>::new()))
        .collect()
        .unwrap();
    validate_batch(&output.batches()[0]).unwrap();
    assert_eq!(output.batches()[0].width(), 0);
}
