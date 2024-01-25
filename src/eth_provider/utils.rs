use std::fmt::LowerHex;

use mongodb::bson::{doc, Document};

/// Converts an iterator of `Into<D>` into a `Vec<D>`.
pub(crate) fn iter_into<D, S: Into<D>>(iter: impl IntoIterator<Item = S>) -> Vec<D> {
    iter.into_iter().map(Into::into).collect::<Vec<_>>()
}

/// Converts a key and value into a MongoDB filter.
pub(crate) fn into_filter<T>(key: &str, value: T, width: usize) -> Document
where
    T: LowerHex,
{
    doc! {key: format!("0x{:0width$x}", value, width = width)}
}
