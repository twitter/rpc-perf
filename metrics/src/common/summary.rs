use core::time::Duration;

#[derive(Clone)]
pub enum Summary {
    Histogram(u64, u32, Option<Duration>),
}
