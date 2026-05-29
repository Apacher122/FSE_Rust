//! Internal retained-leaf execution report types.
//!
//! This module separates retained-leaf report data from reusable result-slot
//! writing while preserving the existing retained report API.

mod batch;
mod leaf;
mod result_slots;

pub(crate) use self::batch::RetainedLeafBatchExecutionReport;
pub(crate) use self::leaf::RetainedLeafExecutionReport;
