//! Fractal Semantic Encoding runtime.
//!
//! This crate contains the storage, construction, and query execution code for
//! the FSE reference runtime.
//!
//! # Formal Mapping
//!
//! The table lists the public runtime items used for the main FSE terms.
//!
//! | FSE term | Runtime item |
//! | --- | --- |
//! | Dataset vector $x \in \mathbb{R}^d$ | [`math::Vector`] |
//! | Scalar coordinate value | [`math::Scalar`] |
//! | Partition $P_k = (D_k, \mu_k, B_k, \Delta_k)$ | [`storage::PartitionNode`] |
//! | Centroid $\mu_k$ | [`storage::PartitionNode::centroid`] |
//! | Bounded support region $B_k$ | [`math::BoundingBox`] |
//! | Residual encoding $\Delta_k(x) = x - \mu_k$ | [`math::ResidualBlock`] |
//! | FSE hierarchy $\mathcal{F}$ | [`storage::FSEIndex`] |
//! | Archive manifest metadata | [`persistence::FSEArchiveManifest`] |
//! | Query region $Q$ | [`query::QueryRegion`] |
//! | Geometric selection $\sigma_G$ / $RT(Q)$ | [`query::traverse`] |
//! | Reconstruction operator $\Phi_k(\Delta) = \mu_k + \Delta$ | [`query::reconstruct_row_into`] |
//! | Logical evaluation $\sigma_L$ | [`query::evaluate_query`] |
//! | Execution operator $E(Q, \mathcal{F})$ | [`query::execute_query`] |
//!
//! # Execution
//!
//! Query execution uses three ordered stages:
//!
//! 1. [`query::traverse`] selects retained leaf partitions from bounding boxes.
//! 2. [`query::reconstruct_row_into`] reconstructs retained residual rows.
//! 3. [`query::evaluate_query`] filters reconstructed coordinates.
//!
//! Traversal reads [`math::BoundingBox`] metadata. Reconstruction reads a leaf
//! centroid and [`math::ResidualBlock`]. Predicate evaluation reads
//! reconstructed coordinate values and [`query::QueryRegion`].
//!
//! # Construction
//!
//! [`build::FSEBuilder`] builds an [`storage::FSEIndex`] from coordinate
//! vectors. The builder computes partition centroids, bounding boxes, residual
//! blocks, and child partitions.
//!
//! [`build::metrics`] reports split quality, sibling overlap, and structural
//! density for constructed indexes.
//!
//! # Invariants
//!
//! The runtime relies on these invariants:
//!
//! - leaf partitions own disjoint row sets;
//! - each bounding box encloses the records represented by its partition;
//! - residual reconstruction computes `centroid + residual`;
//! - query execution does not mutate centroids, bounds, residuals, or topology;
//! - exact predicates run after reconstruction.
//!
//! Parallel execution and future optimized kernels must return the same query
//! results as serial execution.

pub mod benchmark;
pub mod build;
pub mod data;
pub mod encoding;
pub mod import;
pub mod math;
pub mod persistence;
pub mod query;
pub mod storage;

#[cfg(test)]
mod tests;
