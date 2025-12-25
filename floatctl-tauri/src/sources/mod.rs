//! Data sources for the hierarchical browser
//!
//! Each source provides items matching the common Item schema,
//! enabling uniform rendering regardless of underlying data store.

pub mod bbs;
pub mod filesystem;
pub mod jobs;
pub mod search;
pub mod static_list;
