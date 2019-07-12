use crate::counter::*;
use atomics::*;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Histogram<T>
where
    T: Counter + Unsigned,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating,
{
    exact: AtomicU64,
    max: AtomicU64,
    buckets: Vec<T>,
    index: Vec<AtomicU64>,
    too_high: AtomicU64,
    precision: AtomicU32,
    samples: Option<Arc<Mutex<VecDeque<Sample<<T as AtomicPrimitive>::Primitive>>>>>,
    window: Option<Arc<Mutex<Duration>>>,
    capacity: Option<AtomicUsize>,
}

enum Direction {
    Decrement,
    Increment,
}

struct Sample<T> {
    value: u64,
    count: T,
    time: Instant,
    direction: Direction,
}

pub struct Bucket<T> {
    min: u64,
    max: u64,
    value: u64,
    count: T,
}

impl<T> Bucket<T>
where
    T: Copy,
{
    pub fn min(&self) -> u64 {
        self.min
    }
    pub fn max(&self) -> u64 {
        self.max
    }
    pub fn value(&self) -> u64 {
        self.value
    }
    pub fn count(&self) -> T {
        self.count
    }
    pub fn width(&self) -> u64 {
        self.max - self.min
    }
}

impl<T> Histogram<T>
where
    T: Counter + Unsigned,
    u64: std::convert::From<<T as AtomicPrimitive>::Primitive>,
    <T as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating,
{
    pub fn new(
        max: u64,
        precision: u32,
        window: Option<Duration>,
        capacity: Option<usize>,
    ) -> Self {
        let mut histogram: Histogram<T> = Histogram {
            exact: AtomicU64::new(10_u64.pow(precision)),
            max: AtomicU64::new(max),
            buckets: Vec::new(),
            index: Vec::new(),
            too_high: AtomicU64::new(0),
            precision: AtomicU32::new(precision),
            samples: None,
            window: None,
            capacity: None,
        };
        if let Some(window) = window {
            histogram.window = Some(Arc::new(Mutex::new(window)));
            if let Some(capacity) = capacity {
                histogram.samples = Some(Arc::new(Mutex::new(VecDeque::with_capacity(capacity))));
                histogram.capacity = Some(AtomicUsize::new(capacity));
            } else {
                histogram.samples = Some(Arc::new(Mutex::new(VecDeque::with_capacity(1))));
            }
        } else if let Some(capacity) = capacity {
            histogram.samples = Some(Arc::new(Mutex::new(VecDeque::with_capacity(capacity))));
            histogram.capacity = Some(AtomicUsize::new(capacity));
        }
        for _ in 0..=histogram.get_index(max).unwrap() {
            histogram.buckets.push(T::default());
        }
        histogram.buckets.shrink_to_fit();
        for _ in 0..=(histogram.get_index(max).unwrap() / 100) {
            histogram.index.push(AtomicU64::default());
        }
        histogram.index.shrink_to_fit();
        histogram
    }

    /// Returns the total size of the `Histogram` in bytes
    pub fn size(&self) -> usize {
        let mut total_size = 0;
        // add the struct overhead
        total_size += std::mem::size_of::<Histogram<T>>();
        // add the bucket storage
        total_size += std::mem::size_of::<T>() * self.buckets.capacity();
        // add the index storage
        total_size += std::mem::size_of::<AtomicU64>() * self.index.capacity();
        // add the samples storage
        if let Some(samples) = &self.samples {
            let samples = samples.lock();
            total_size += std::mem::size_of::<Sample<T>>() * samples.capacity();
        }
        total_size
    }

    // Internal function to get the index for a given value
    fn get_index(&self, value: u64) -> Result<usize, ()> {
        if value > self.max.get() {
            Err(())
        } else if value <= self.exact.get() {
            Ok(value as usize)
        } else {
            let exact = self.exact.get() as usize;
            let power = (value as f64).log10().floor() as u32;
            let divisor = 10_u64.pow((power - self.precision.get()) as u32 + 1);
            let power_offset =
                (0.9 * f64::from(exact as u32 * (power - self.precision.get()))) as usize;
            let remainder = value / divisor as u64;
            let shift = exact / 10;
            let index = exact + power_offset + remainder as usize - shift;
            Ok(index)
        }
    }

    fn get_min_value(&self, index: usize) -> Result<u64, ()> {
        if index >= self.buckets.len() {
            Err(())
        } else if (index as u64) <= self.exact.get() {
            Ok(index as u64)
        } else if index == self.buckets.len() - 1 {
            Ok(self.max.get())
        } else {
            let shift = 10_usize.pow(self.precision.get() - 1);
            let base_offset = 10_usize.pow(self.precision.get());
            let power = self.precision.get() as usize
                + (index - base_offset) / (9 * 10_usize.pow(self.precision.get() - 1));
            let power_offset = (0.9
                * (10_usize.pow(self.precision.get()) * (power - self.precision.get() as usize))
                    as f64) as usize;
            let value = (index + shift - base_offset - power_offset) as u64
                * 10_u64.pow((power - self.precision.get() as usize + 1) as u32);
            Ok(value)
        }
    }

    fn get_max_value(&self, index: usize) -> Result<u64, ()> {
        if index == self.buckets.len() - 1 {
            Ok(self.max.get() + 1)
        } else {
            Ok(self.get_min_value(index + 1).unwrap())
        }
    }

    // Internal function to get the bucket at a given index
    fn get_bucket(&self, index: usize) -> Option<Bucket<<T as AtomicPrimitive>::Primitive>> {
        if let Some(counter) = self.buckets.get(index) {
            Some(Bucket {
                min: self.get_min_value(index).unwrap(),
                max: self.get_max_value(index).unwrap(),
                value: self.get_value(index).unwrap(),
                count: counter.get(),
            })
        } else {
            None
        }
    }

    fn get_value(&self, index: usize) -> Result<u64, ()> {
        self.get_max_value(index).map(|v| v - 1)
    }

    pub fn increment(&self, value: u64, count: <T as AtomicPrimitive>::Primitive) {
        match self.get_index(value) {
            Ok(index) => {
                self.buckets[index].saturating_add(count);
                self.index[index / 100].saturating_add(u64::from(count));
                if let Some(samples) = &self.samples {
                    let time = Instant::now();
                    self.trim(time);
                    let mut samples = samples.lock();
                    samples.push_back(Sample {
                        value,
                        count,
                        time,
                        direction: Direction::Increment,
                    });
                }
            }
            Err(_) => {
                self.too_high.saturating_add(u64::from(count));
            }
        }
    }

    pub fn decrement(&self, value: u64, count: <T as AtomicPrimitive>::Primitive) {
        match self.get_index(value) {
            Ok(index) => {
                self.buckets[index].saturating_sub(count);
                self.index[index / 100].saturating_sub(u64::from(count));
                if let Some(samples) = &self.samples {
                    let time = Instant::now();
                    self.trim(time);
                    let mut samples = samples.lock();
                    samples.push_back(Sample {
                        value,
                        count,
                        time,
                        direction: Direction::Decrement,
                    });
                }
            }
            Err(_) => {
                self.too_high.saturating_sub(u64::from(count));
            }
        }
    }

    pub fn clear(&self) {
        if let Some(samples) = &self.samples {
            let mut samples = samples.lock();
            samples.clear();
        }
        for i in 0..self.buckets.len() {
            self.buckets[i].set(<T as AtomicPrimitive>::Primitive::default());
        }
        for i in 0..self.index.len() {
            self.index[i].set(0);
        }
        self.too_high.set(0);
    }

    fn trim(&self, time: Instant) {
        if let Some(samples) = &self.samples {
            if let Some(window) = &self.window {
                let window = *window.lock();
                let mut samples = samples.lock();
                while let Some(sample) = samples.pop_front() {
                    let age = time - sample.time;
                    if age > window {
                        match self.get_index(sample.value) {
                            Ok(index) => match sample.direction {
                                Direction::Decrement => {
                                    self.buckets[index].saturating_add(sample.count);
                                    self.index[index / 100].saturating_add(u64::from(sample.count));
                                }
                                Direction::Increment => {
                                    self.buckets[index].saturating_sub(sample.count);
                                    self.index[index / 100].saturating_sub(u64::from(sample.count));
                                }
                            },
                            Err(_) => match sample.direction {
                                Direction::Decrement => {
                                    self.too_high.saturating_add(u64::from(sample.count));
                                }
                                Direction::Increment => {
                                    self.too_high.saturating_sub(u64::from(sample.count));
                                }
                            },
                        }
                    } else {
                        samples.push_front(sample);
                        break;
                    }
                }
                samples.shrink_to_fit();
            }
            if let Some(capacity) = &self.capacity {
                let capacity = capacity.get();
                let mut samples = samples.lock();
                while samples.len() > capacity {
                    if let Some(sample) = samples.pop_front() {
                        match self.get_index(sample.value) {
                            Ok(index) => match sample.direction {
                                Direction::Decrement => {
                                    self.buckets[index].saturating_add(sample.count);
                                    self.index[index / 100].saturating_add(u64::from(sample.count));
                                }
                                Direction::Increment => {
                                    self.buckets[index].saturating_sub(sample.count);
                                    self.index[index / 100].saturating_sub(u64::from(sample.count));
                                }
                            },
                            Err(_) => match sample.direction {
                                Direction::Decrement => {
                                    self.too_high.saturating_add(u64::from(sample.count));
                                }
                                Direction::Increment => {
                                    self.too_high.saturating_sub(u64::from(sample.count));
                                }
                            },
                        }
                    }
                }
                samples.shrink_to_fit();
            }
        }
    }

    pub fn total_count(&self) -> u64 {
        if self.samples.is_some() {
            let time = Instant::now();
            self.trim(time);
        }
        let mut total = 0;
        for i in 0..self.index.len() {
            total += self.index[i].get();
        }
        total += self.too_high.get();
        total
    }

    pub fn percentile(&self, percentile: f64) -> Option<u64> {
        let total = self.total_count();
        if total == 0 {
            None
        } else {
            let mut need = (percentile * total as f64).ceil() as u64;
            if need == 0 {
                need = 1;
            }
            let mut have = 0;
            for i in 0..self.index.len() {
                let count = self.index[i].get();
                if have + count >= need {
                    let index = i * 100;
                    for j in index..(index + 100) {
                        have += u64::from(self.buckets[j].get());
                        if have >= need {
                            return Some(self.get_value(j).unwrap());
                        }
                    }
                }
                have += count;
            }
            Some(self.max.get())
        }
    }

    pub fn too_high(&self) -> u64 {
        self.too_high.get()
    }

    pub fn mean(&self) -> u64 {
        let mut result = 0;
        for bucket in self.into_iter() {
            result += u64::from(bucket.count) * bucket.value;
        }
        result / self.total_count()
    }

    pub fn mode(&self) -> u64 {
        let mut count = 0;
        let mut value = 0;
        for bucket in self.into_iter() {
            if u64::from(bucket.count()) > count {
                count = u64::from(bucket.count());
                value = bucket.value();
            }
        }
        value
    }
}

pub struct Iter<'a, C>
where
    C: Counter + Unsigned,
    <C as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating,
{
    inner: &'a Histogram<C>,
    index: usize,
}

impl<'a, C> Iter<'a, C>
where
    C: Counter + Unsigned,
    <C as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating,
{
    fn new(inner: &'a Histogram<C>) -> Iter<'a, C> {
        Iter { inner, index: 0 }
    }
}

impl<'a, C> Iterator for Iter<'a, C>
where
    C: Counter + Unsigned,
    <C as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating,
    u64: From<<C as AtomicPrimitive>::Primitive>,
{
    type Item = Bucket<<C as AtomicPrimitive>::Primitive>;

    fn next(&mut self) -> Option<Bucket<<C as AtomicPrimitive>::Primitive>> {
        let bucket = self.inner.get_bucket(self.index);
        self.index += 1;
        bucket
    }
}

impl<'a, C> IntoIterator for &'a Histogram<C>
where
    C: Counter + Unsigned,
    <C as AtomicPrimitive>::Primitive: Default + PartialEq + Copy + Saturating,
    u64: From<<C as AtomicPrimitive>::Primitive>,
{
    type Item = Bucket<<C as AtomicPrimitive>::Primitive>;
    type IntoIter = Iter<'a, C>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_latched() {
        let h = Histogram::<AtomicU64>::new(100, 3, None, None);
        assert_eq!(h.total_count(), 0);
        for i in 1..=100 {
            let _ = h.increment(i, 1);
            assert_eq!(h.total_count(), i);
        }
        assert_eq!(h.percentile(0.0), Some(1));
        assert_eq!(h.percentile(0.05), Some(5));
        assert_eq!(h.percentile(0.1), Some(10));
        assert_eq!(h.percentile(0.25), Some(25));
        assert_eq!(h.percentile(0.50), Some(50));
        assert_eq!(h.percentile(0.75), Some(75));
        assert_eq!(h.percentile(0.90), Some(90));
        assert_eq!(h.percentile(0.95), Some(95));
        assert_eq!(h.percentile(0.99), Some(99));
        assert_eq!(h.percentile(0.999), Some(100));
        assert_eq!(h.percentile(1.0), Some(100));
        h.clear();
        assert_eq!(h.percentile(0.0), None);
        assert_eq!(h.total_count(), 0);
        assert_eq!(h.size(), 936);
    }

    #[test]
    fn size() {
        let h = Histogram::<AtomicU8>::new(1_000_000_000, 3, None, None);
        assert_eq!(h.size() / 1024, 6); // ~6KB

        let h = Histogram::<AtomicU16>::new(1_000_000_000, 3, None, None);
        assert_eq!(h.size() / 1024, 13); // ~13KB

        let h = Histogram::<AtomicU32>::new(1_000_000_000, 3, None, None);
        assert_eq!(h.size() / 1024, 25); // ~25KB

        let h = Histogram::<AtomicU32>::new(60_000_000_000, 3, None, None);
        assert_eq!(h.size() / 1024, 31); // ~31KB

        let h = Histogram::<AtomicU64>::new(1_000_000_000, 3, None, None);
        assert_eq!(h.size() / 1024, 50); // ~50KB

        let h =
            Histogram::<AtomicU64>::new(1_000_000_000, 3, Some(<Duration>::from_millis(1)), None);
        assert_eq!(h.size() / 1024, 50); // ~50KB

        let h = Histogram::<AtomicU64>::new(
            1_000_000_000,
            3,
            Some(<Duration>::from_millis(1)),
            Some(60),
        );
        assert!(h.size() / 1024 >= 52); // ~52KB
        assert!(h.size() / 1024 <= 53); // ~52KB
    }

    #[test]
    fn basic_moving() {
        let h = Histogram::<AtomicU64>::new(100, 3, Some(<Duration>::from_millis(1)), None);
        assert_eq!(h.total_count(), 0);
        for i in 1..100 {
            let _ = h.increment(i, 1);
            assert_eq!(h.total_count(), 1);
            assert_eq!(h.percentile(0.0), Some(i));
            assert_eq!(h.percentile(1.0), Some(i));
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(h.total_count(), 0);
    }

    #[test]
    fn basic_capacity() {
        let h = Histogram::<AtomicU64>::new(100, 3, None, Some(1));
        assert_eq!(h.total_count(), 0);
        for i in 1..100 {
            let _ = h.increment(i, 1);
            assert_eq!(h.total_count(), 1);
            assert_eq!(h.percentile(0.0), Some(i));
            assert_eq!(h.percentile(1.0), Some(i));
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(h.total_count(), 1);
    }

    #[test]
    fn basic_moving_capacity() {
        let h = Histogram::<AtomicU64>::new(100, 3, Some(<Duration>::from_millis(1)), Some(1));
        assert_eq!(h.total_count(), 0);
        for i in 1..100 {
            let _ = h.increment(i, 1);
            assert_eq!(h.total_count(), 1);
            assert_eq!(h.percentile(0.0), Some(i));
            assert_eq!(h.percentile(1.0), Some(i));
        }
        assert_eq!(h.total_count(), 1);
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(h.total_count(), 0);
    }
}
