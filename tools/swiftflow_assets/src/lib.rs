mod catalog;
mod contents;
mod flatten;

pub use catalog::{Catalog, Error, ImageSet, Result, Scale, Slot};
pub use contents::{Contents, ImageEntry, Info};
pub use flatten::{flatten, FlattenReport};
