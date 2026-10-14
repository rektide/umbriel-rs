#![forbid(unsafe_code)]
//! Owned Polars-native Arrow snapshots, independent of state and query engines.

/// The exact native Arrow ecosystem selected by this emitter.
pub use polars_arrow;

use std::{error::Error, fmt, sync::Arc};

use polars_arrow::{
    array::{ArrayRef, BooleanArray, PrimitiveArray, Utf8ViewArray},
    datatypes::{ArrowDataType, ArrowSchema, Field},
};
use umbriel_protocol::model::{Window, Workspace};

pub use polars_arrow::record_batch::RecordBatch;

/// Version of the complete wire-named windows/workspaces schemas.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Windows,
    Workspaces,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Replacement {
    Complete,
}

/// Metadata belongs to the snapshot, including when there are no rows.
/// `provenance` is caller-owned local observation information, not a server
/// transaction, atomic cross-family revision, or resumable cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMetadata<P> {
    pub family: Family,
    pub schema_version: u32,
    pub replacement: Replacement,
    pub provenance: P,
}

/// A complete replacement with owned column buffers and checked schema/array
/// agreement. No raw-batch constructor or mutable batch access is exposed.
#[derive(Debug, Clone)]
pub struct SnapshotBatch<P> {
    metadata: SnapshotMetadata<P>,
    batch: RecordBatch,
}

impl<P> SnapshotBatch<P> {
    pub fn metadata(&self) -> &SnapshotMetadata<P> {
        &self.metadata
    }

    pub fn batch(&self) -> &RecordBatch {
        &self.batch
    }

    /// Consumes the invariant-preserving envelope. A raw batch extracted here
    /// cannot be reimported into this type through an unchecked constructor.
    pub fn into_parts(self) -> (SnapshotMetadata<P>, RecordBatch) {
        (self.metadata, self.batch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchError(String);

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for BatchError {}

/// Checks the structural invariant that upstream `RecordBatch::try_new` does
/// not completely check. This is not an import API or a promise that every
/// external dtype is supported by Polars' DataFrame conversion.
pub fn validate_batch(batch: &RecordBatch) -> Result<(), BatchError> {
    validate_parts(batch.height(), batch.schema(), batch.arrays())
}

fn validate_parts(
    height: usize,
    schema: &ArrowSchema,
    arrays: &[ArrayRef],
) -> Result<(), BatchError> {
    if schema.len() != arrays.len() {
        return Err(BatchError("field/array count mismatch".into()));
    }
    for ((name, field), array) in schema.iter().zip(arrays) {
        if name != &field.name {
            return Err(BatchError(format!(
                "schema key/field name mismatch: {name}"
            )));
        }
        if field.dtype() != array.dtype() {
            return Err(BatchError(format!("schema/array dtype mismatch: {name}")));
        }
        if array.len() != height {
            return Err(BatchError(format!("array height mismatch: {name}")));
        }
        if !field.is_nullable && array.null_count() != 0 {
            return Err(BatchError(format!("non-null field contains nulls: {name}")));
        }
    }
    Ok(())
}

fn finish<P>(
    family: Family,
    provenance: P,
    height: usize,
    schema: ArrowSchema,
    arrays: Vec<ArrayRef>,
) -> Result<SnapshotBatch<P>, BatchError> {
    validate_parts(height, &schema, &arrays)?;
    let batch = RecordBatch::try_new(height, Arc::new(schema), arrays)
        .map_err(|error| BatchError(error.to_string()))?;
    Ok(SnapshotBatch {
        metadata: SnapshotMetadata {
            family,
            schema_version: SCHEMA_VERSION,
            replacement: Replacement::Complete,
            provenance,
        },
        batch,
    })
}

fn strings<'a>(values: impl Iterator<Item = &'a str>) -> ArrayRef {
    // The builder copies strings into owned view/buffer storage, not row borrows.
    Box::new(Utf8ViewArray::from_slice_values(values.collect::<Vec<_>>()))
}

fn integers(values: impl Iterator<Item = i32>) -> ArrayRef {
    Box::new(PrimitiveArray::<i32>::from_vec(values.collect()))
}

fn booleans(values: impl Iterator<Item = bool>) -> ArrayRef {
    Box::new(BooleanArray::from_slice(values.collect::<Vec<_>>()))
}

// Schema and emission share one declaration; finish still checks the actual
// array dtypes, counts, names, nullability and heights before publication.
macro_rules! family {
    ($emit:ident, $schema:ident, $row:ty, $family:ident, {
        $($name:literal => $dtype:ident, $builder:ident, $value:expr;)*
    }) => {
        pub fn $schema() -> ArrowSchema {
            [$(Field::new($name.into(), ArrowDataType::$dtype, false)),*]
                .into_iter().map(|field| (field.name.clone(), field)).collect()
        }

        /// Emits one complete replacement directly from protocol rows.
        /// All buffers are owned; neither rows nor receive bytes are retained.
        pub fn $emit<P>(rows: &[$row], provenance: P) -> Result<SnapshotBatch<P>, BatchError> {
            let arrays: Vec<ArrayRef> = vec![$($builder(rows.iter().map($value))),*];
            finish(Family::$family, provenance, rows.len(), $schema(), arrays)
        }
    };
}

family!(windows, windows_schema, Window, Windows, {
    "active" => Boolean, booleans, |r| r.active;
    "app_id" => Utf8View, strings, |r| r.app_id.as_str();
    "content_type" => Utf8View, strings, |r| r.content_type.as_str();
    "floating" => Boolean, booleans, |r| r.floating;
    "focused" => Boolean, booleans, |r| r.focused;
    "h" => Int32, integers, |r| r.h;
    "id" => Utf8View, strings, |r| r.id.as_str();
    "pid" => Int32, integers, |r| r.pid;
    "scratchpad" => Utf8View, strings, |r| r.scratchpad.as_str();
    "title" => Utf8View, strings, |r| r.title.as_str();
    "urgent" => Boolean, booleans, |r| r.urgent;
    "w" => Int32, integers, |r| r.w;
    "workspace" => Utf8View, strings, |r| r.workspace.as_str();
    "x" => Int32, integers, |r| r.x;
    "xdg_tag" => Utf8View, strings, |r| r.xdg_tag.as_str();
    "xwayland" => Boolean, booleans, |r| r.xwayland;
    "y" => Int32, integers, |r| r.y;
});

family!(workspaces, workspaces_schema, Workspace, Workspaces, {
    "active" => Boolean, booleans, |r| r.active;
    "focused" => Boolean, booleans, |r| r.focused;
    "id" => Utf8View, strings, |r| r.id.as_str();
    "index" => Int32, integers, |r| r.index;
    "layout" => Utf8View, strings, |r| r.layout.as_str();
    "name" => Utf8View, strings, |r| r.name.as_str();
    "named" => Boolean, booleans, |r| r.named;
    "occupied" => Boolean, booleans, |r| r.occupied;
    "output" => Utf8View, strings, |r| r.output.as_str();
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dtype_mismatch_is_rejected_even_when_upstream_accepts_it() {
        let schema = [Field::new("x".into(), ArrowDataType::Utf8View, false)]
            .into_iter()
            .map(|f| (f.name.clone(), f))
            .collect();
        let batch =
            RecordBatch::try_new(1, Arc::new(schema), vec![integers([1].into_iter())]).unwrap();
        assert!(validate_batch(&batch).is_err());
    }
}
