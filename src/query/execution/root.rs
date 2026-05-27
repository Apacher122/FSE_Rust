//! Shared root query classification helpers.

use crate::query::QueryRegion;
use crate::query::region::QueryBoundsClassification;
use crate::storage::FSEIndex;

/// Validates query dimensionality and classifies the query against root bounds.
///
/// # Runtime Role
///
/// Every public query execution contract starts by proving that the query
/// dimensionality matches the index and then classifying the query against the
/// root bounding region. Keeping that root setup in one helper prevents the
/// owned-result, reusable-result, count-only, and reference-result paths from
/// drifting apart.
///
/// # Formal Reference
///
/// This is the root-level metadata classification for Stage I:
///
/// `Geometry -> Reconstruction -> Logic`.
///
/// # Panics
///
/// Panics when the query dimensionality does not match the index dimensionality.
#[inline]
pub(crate) fn classify_query_root(
    index: &FSEIndex,
    query: &QueryRegion,
) -> QueryBoundsClassification {
    assert_eq!(
        index.dimensions,
        query.dimensions(),
        "query dimensionality must match index dimensionality"
    );

    query.classify_bounds(&index.root_node().bounds)
}
