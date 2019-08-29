// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

mod atomic_counter;
mod atomic_option;
mod atomic_primitive;

pub use crate::atomic_counter::*;
pub use crate::atomic_option::*;
pub use crate::atomic_primitive::*;
pub use core::sync::atomic::Ordering;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usize() {
        let x = AtomicUsize::new(0);
        assert_eq!(x.load(Ordering::SeqCst), 0_usize);
        x.store(42, Ordering::SeqCst);
        assert_eq!(x.load(Ordering::SeqCst), 42_usize);
    }
}
