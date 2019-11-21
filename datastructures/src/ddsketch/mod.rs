//! Different implementations of DDSketch.

mod atomic;
mod dense;

pub use self::atomic::AtomicDDSketch;
pub use self::dense::DenseDDSketch;
