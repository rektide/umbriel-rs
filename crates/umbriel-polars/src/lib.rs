#![forbid(unsafe_code)]
//! Explicit finite lazy execution; no live state, socket ownership or timers.

/// The exact native query ecosystem selected by this integration.
pub use polars;

use std::sync::Arc;

use polars::prelude::{CompatLevel, DataFrame, IntoLazy, LazyFrame, PolarsError, PolarsResult};
use umbriel_arrow::{RecordBatch, SnapshotBatch, SnapshotMetadata, validate_batch};

/// Checked emitter input as a DataFrame with its original observation metadata.
/// Only the emitter's invariant-preserving type is accepted, not arbitrary raw
/// record batches whose schema may disagree with their arrays.
///
/// ```compile_fail
/// fn unchecked(raw: umbriel_arrow::RecordBatch) {
///     let _: umbriel_polars::SnapshotFrame<()> = umbriel_polars::SnapshotFrame::from(raw);
/// }
/// ```
pub struct SnapshotFrame<P> {
    source: SnapshotMetadata<P>,
    frame: DataFrame,
}

impl<P> From<SnapshotBatch<P>> for SnapshotFrame<P> {
    fn from(snapshot: SnapshotBatch<P>) -> Self {
        let (source, batch) = snapshot.into_parts();
        // The private emitter constructor checks dtypes as well as lengths and
        // only emits Polars-supported Boolean, Int32 and Utf8View columns.
        Self {
            source,
            frame: DataFrame::from(batch),
        }
    }
}

impl<P> SnapshotFrame<P> {
    pub fn source(&self) -> &SnapshotMetadata<P> {
        &self.source
    }

    pub fn frame(&self) -> &DataFrame {
        &self.frame
    }

    pub fn into_parts(self) -> (SnapshotMetadata<P>, DataFrame) {
        (self.source, self.frame)
    }

    pub fn lazy(self) -> SnapshotQuery<P> {
        SnapshotQuery {
            source: self.source,
            plan: self.frame.lazy(),
        }
    }
}

/// A finite plan retaining its input provenance. A plan does not observe later
/// snapshots or maintain an incremental query. If a caller adds other inputs
/// in `map`, it must include their provenance in the caller's metadata itself.
pub struct SnapshotQuery<P> {
    source: SnapshotMetadata<P>,
    plan: LazyFrame,
}

impl<P> SnapshotQuery<P> {
    pub fn source(&self) -> &SnapshotMetadata<P> {
        &self.source
    }

    /// Apply caller-selected native lazy expressions without executing them.
    pub fn map(self, transform: impl FnOnce(LazyFrame) -> LazyFrame) -> Self {
        Self {
            source: self.source,
            plan: transform(self.plan),
        }
    }

    /// Explicit blocking finite execution. Run CPU-heavy work outside a socket
    /// receive owner. Rechunking creates one owned native batch, including for
    /// zero rows/columns; it can allocate and is not an incremental engine.
    pub fn collect(self) -> PolarsResult<QueryResult<P>> {
        let frame = self.plan.collect()?;
        let height = frame.height();
        let schema = frame
            .columns()
            .iter()
            .map(|column| column.field().to_arrow(CompatLevel::newest()))
            .collect();
        let arrays = frame.rechunk_into_arrow(CompatLevel::newest());
        let batch = RecordBatch::try_new(height, Arc::new(schema), arrays)?;
        validate_batch(&batch)
            .map_err(|error| PolarsError::ComputeError(error.to_string().into()))?;
        Ok(QueryResult {
            source: self.source,
            batches: vec![batch],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    Derived,
}

/// Derived query output, never a compositor family replacement. `source`
/// describes the complete *input* replacement, not the meaning of output rows.
#[derive(Debug, Clone)]
pub struct QueryResult<P> {
    source: SnapshotMetadata<P>,
    batches: Vec<RecordBatch>,
}

impl<P> QueryResult<P> {
    pub fn kind(&self) -> ResultKind {
        ResultKind::Derived
    }

    pub fn source(&self) -> &SnapshotMetadata<P> {
        &self.source
    }

    pub fn batches(&self) -> &[RecordBatch] {
        &self.batches
    }

    pub fn into_parts(self) -> (SnapshotMetadata<P>, Vec<RecordBatch>) {
        (self.source, self.batches)
    }
}
