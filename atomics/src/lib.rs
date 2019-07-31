mod atomic_counter;
mod atomic_primitive;

pub use crate::atomic_counter::*;
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
