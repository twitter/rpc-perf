// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::histogram::*;

use atomics::*;
use parking_lot::Mutex;
use time::Tm;

use std::convert::From;
use std::sync::Arc;

const SECOND: u64 = 1_000_000_000;

/// A `Slice` holds a `Histogram` which covers a window of time within the time
/// range covered by the `Heatmap`
pub struct Slice<'a, T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    u64: From<T>,
    T: Copy + Default,
{
    begin_utc: Tm,
    end_utc: Tm,
    begin_precise: u64,
    end_precise: u64,
    histogram: &'a Histogram<T>,
}

impl<'a, T> Slice<'a, T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    u64: From<T>,
    T: Copy + Default,
{
    /// Returns the start of the slice in UTC wallclock time
    pub fn begin_utc(&self) -> Tm {
        self.begin_utc
    }

    /// Returns the end of the slice in UTC wallclock time
    pub fn end_utc(&self) -> Tm {
        self.end_utc
    }

    /// Returns the start of the slice as precise count with an arbitrary epoch
    pub fn begin_precise(&self) -> u64 {
        self.begin_precise
    }

    /// Returns the end of the slice as a precise count with an arbitrary epoch
    pub fn end_precise(&self) -> u64 {
        self.end_precise
    }

    /// Access the `Histogram` stored within the `Slice`
    pub fn histogram(&self) -> &Histogram<T> {
        &self.histogram
    }
}

/// A `Heatmap` is used to store multiple `Histogram`s across a span of time
/// with each `Slice` of the `Heatmap` covering a sub-span of the overall span.
/// The number of slices are dictated by the `resolution` in nanoseconds and the
/// `span` of the heatmap in nanoseconds. Each `Histogram` within the `Heatmap`
/// will store from 0..`max` with a specified `precision`.
#[rustfmt::skip]
pub struct Heatmap<T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    u64: From<T>,
    T: Copy + Default,
{
    oldest_begin_precise: Atomic<u64>,  // this is the start time of oldest slice
    newest_begin_precise: Atomic<u64>,  // start time of newest slice
    newest_end_precise: Atomic<u64>,    // end time of the oldest slice
    oldest_begin_utc: Arc<Mutex<Tm>>, // relates start time of oldest slice to wall-clock
    resolution: Atomic<u64>,            // number of NS per slice
    slices: Vec<Histogram<T>>,        // stores the `Histogram`s
    offset: Atomic<usize>,              // indicates which `Histogram` is the oldest
}

impl<T> Heatmap<T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    u64: From<T>,
    T: Copy + Default,
{
    /// Create a new `Heatmap` which will hold values from 0..`max` with a
    /// specified `precision`. Use `resolution` to specify the time-domain
    /// resolution in nanoseconds. Use `span` to specify the overall window of
    /// time to be covered by the `Heatmap`.
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// // creates a heatmap for storing 32-bit counters of values from
    /// // 0..1_000_000 with each second in its own histogram for the past
    /// // five minutes
    /// let x = Heatmap::<AtomicU32>::new(1_000_000, 3, SECOND, 300 * SECOND);
    /// ```
    pub fn new(max: u64, precision: u32, resolution: u64, span: u64) -> Self {
        // build the Histograms
        let num_slices = span / resolution;
        let mut slices = Vec::with_capacity(num_slices as usize);
        for _ in 0..num_slices {
            slices.push(Histogram::new(max, precision, None, None));
        }

        // get time and align with previous top of minute
        let now_utc = time::now_utc();
        let now_precise = time::precise_time_ns();
        let adjusted_precise =
            now_precise - now_utc.tm_nsec as u64 - now_utc.tm_sec as u64 * SECOND; // set backward to top of minute
        let adjusted_utc =
            now_utc - time::Duration::nanoseconds((now_precise - adjusted_precise) as i64); // set backward to top of minute

        Heatmap {
            oldest_begin_precise: Atomic::<u64>::new(adjusted_precise),
            newest_begin_precise: Atomic::<u64>::new(adjusted_precise + span - resolution),
            newest_end_precise: Atomic::<u64>::new(adjusted_precise + span),
            oldest_begin_utc: Arc::new(Mutex::new(adjusted_utc)),
            offset: Default::default(),
            resolution: Atomic::<u64>::new(resolution),
            slices,
        }
    }

    // internal function to calculate the index for a time
    fn get_index(&self, time: u64) -> Option<usize> {
        if self.oldest_begin_precise.load(Ordering::Relaxed) < time && time < self.newest_end_precise.load(Ordering::Relaxed) {
            let mut index = ((time - self.oldest_begin_precise.load(Ordering::Relaxed)) / self.resolution.load(Ordering::Relaxed))
                as usize
                + self.offset.load(Ordering::Relaxed) as usize;
            if index >= self.slices.len() {
                index -= self.slices.len();
            }
            Some(index)
        } else {
            None
        }
    }

    // internal function to tick forward by one slice
    fn try_tick(&self) -> Result<(), ()> {
        let current_offset = self.offset.load(Ordering::Relaxed);
        let next_offset = if current_offset == self.slices.len() - 1 {
            0
        } else {
            current_offset + 1
        };
        // get a lock
        let current = *self.oldest_begin_utc.lock();
        if self.offset.compare_and_swap(current_offset, next_offset, Ordering::Relaxed) == current_offset {
            let resolution = self.resolution.load(Ordering::Relaxed);
            self.oldest_begin_precise.fetch_add(resolution, Ordering::Relaxed);
            self.newest_begin_precise.fetch_add(resolution, Ordering::Relaxed);
            self.newest_end_precise.fetch_add(resolution, Ordering::Relaxed);
            *self.oldest_begin_utc.lock() = current + time::Duration::nanoseconds(resolution as i64);
            Ok(())
        } else {
            Err(())
        }
    }

    // Internal function to get the Histogram at a given index
    fn get_histogram(&self, index: usize) -> Option<&Histogram<T>> {
        if let Some(h) = self.slices.get(index) {
            Some(h)
        } else {
            None
        }
    }

    // Internal function to get the current offset of the `Heatmap`
    fn offset(&self) -> usize {
        self.offset.load(Ordering::Relaxed) as usize
    }

    /// Increment a `time`-`value` pair by `count`
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU32>::new(1_000_000, 3, SECOND, 300 * SECOND);
    /// x.increment(precise_time_ns(), 100, 1);
    /// ```
    pub fn increment(&self, time: u64, value: u64, count: T) {
        if let Some(index) = self.get_index(time) {
            self.slices[index].increment(value, count);
        }
    }

    /// Decrement a `time`-`value` pair by `count`
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU32>::new(1_000_000, 3, SECOND, 300 * SECOND);
    /// x.decrement(precise_time_ns(), 100, 1);
    /// ```
    pub fn decrement(&self, time: u64, value: u64, count: T) {
        if let Some(index) = self.get_index(time) {
            self.slices[index].decrement(value, count);
        }
    }

    /// Moves the window forward as-needed, dropping older histograms
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// use std::time::Duration;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU32>::new(1_000_000, 3, SECOND, 2 * SECOND);
    /// x.latch();
    /// x.increment(precise_time_ns(), 100, 1);
    /// assert_eq!(x.total_count(), 1);
    /// std::thread::sleep(Duration::new(2, 0));
    /// x.latch();
    /// assert_eq!(x.total_count(), 0);
    /// ```
    pub fn latch(&self) {
        let time = time::precise_time_ns();

        // we only need to extend the Heatmap if we're currently writing to latest slice
        while time >= self.newest_begin_precise.load(Ordering::Relaxed) {
            let _ = self.try_tick();
        }
    }

    /// Get the total count for all samples stored in the heatmap
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU32>::new(1_000_000, 3, SECOND, 60 * SECOND);
    /// x.increment(precise_time_ns(), 100, 1);
    /// assert_eq!(x.total_count(), 1);
    /// ```
    pub fn total_count(&self) -> u64 {
        let mut count = 0;
        for histogram in &self.slices {
            count += histogram.total_count();
        }
        count
    }

    /// Get the maximum number of samples in any `time`-`value` pair
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU64>::new(1_000_000, 2, SECOND, 60 * SECOND);
    /// for v in 0..100 {
    ///    x.increment(precise_time_ns(), v, v);
    /// }
    /// assert_eq!(x.highest_count(), 99);
    /// ```
    pub fn highest_count(&self) -> u64 {
        let mut highest_count = 0;
        for histogram in self.slices.iter() {
            for bucket in histogram {
                if u64::from(bucket.count()) > highest_count {
                    highest_count = u64::from(bucket.count());
                }
            }
        }
        highest_count
    }

    /// Return the number of `Slice`s within the `Heatmap`
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU64>::new(1_000_000, 2, SECOND, 60 * SECOND);
    /// assert_eq!(x.slices(), 60);
    /// ```
    pub fn slices(&self) -> usize {
        self.slices.len()
    }

    /// Return the number of `Bucket`s within each `Slice`
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU64>::new(1_000_000, 2, SECOND, 60 * SECOND);
    /// assert_eq!(x.buckets(), 461);
    /// ```
    pub fn buckets(&self) -> usize {
        self.slices[0].into_iter().count()
    }

    /// Return the beginning UTC wallclock time covered in the `Heatmap`
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU64>::new(1_000_000, 2, SECOND, 60 * SECOND);
    /// println!("begin time: {:?}", x.begin_utc());
    /// ```
    pub fn begin_utc(&self) -> Tm {
        *self.oldest_begin_utc.lock()
    }

    /// Returns the beginning timestamp from an arbitrary epoch
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU64>::new(1_000_000, 2, SECOND, 60 * SECOND);
    /// assert!(x.begin_precise() < precise_time_ns());
    /// ```
    pub fn begin_precise(&self) -> u64 {
        self.oldest_begin_precise.load(Ordering::Relaxed)
    }

    /// Return the number of nanoseconds stored in each `Slice`
    ///
    /// # Examples
    ///
    /// ```
    /// use datastructures::*;
    /// use time::*;
    ///
    /// const SECOND: u64 = 1_000_000_000;
    ///
    /// let x = Heatmap::<AtomicU64>::new(1_000_000, 2, SECOND, 60 * SECOND);
    /// assert_eq!(x.resolution(), SECOND);
    /// ```
    pub fn resolution(&self) -> u64 {
        self.resolution.load(Ordering::Relaxed)
    }
}

pub struct Iter<'a, T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    u64: From<T>,
    T: Copy + Default,
{
    inner: &'a Heatmap<T>,
    index: usize,
}

impl<'a, T> Iter<'a, T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    u64: From<T>,
    T: Copy + Default,
{
    fn new(inner: &'a Heatmap<T>) -> Iter<'a, T> {
        Iter { inner, index: 0 }
    }
}

impl<'a, T> Iterator for Iter<'a, T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    u64: From<T>,
    T: Copy + Default,
{
    type Item = Slice<'a, T>;

    fn next(&mut self) -> Option<Slice<'a, T>> {
        if self.index >= self.inner.slices() {
            None
        } else {
            let mut index = self.index + self.inner.offset();
            if index >= self.inner.slices() {
                index -= self.inner.slices();
            }
            let heatmap_begin_precise = self.inner.begin_precise();
            let begin_precise = heatmap_begin_precise + self.index as u64 * self.inner.resolution();
            let heatmap_begin_utc = self.inner.begin_utc();
            self.index += 1;
            Some(Slice {
                begin_precise,
                end_precise: begin_precise + self.inner.resolution(),
                begin_utc: heatmap_begin_utc
                    + time::Duration::nanoseconds((begin_precise - heatmap_begin_precise) as i64),
                end_utc: heatmap_begin_utc
                    + time::Duration::nanoseconds(
                        (begin_precise + self.inner.resolution() - heatmap_begin_precise) as i64,
                    ),
                histogram: self.inner.get_histogram(index).unwrap(),
            })
        }
    }
}

impl<'a, T> IntoIterator for &'a Heatmap<T>
where
    Atomic<T>: Default + Unsigned + AtomicPrimitive<T> + AtomicSaturatingAdd<T> + AtomicSaturatingSub<T>,
    u64: From<T>,
    T: Copy + Default,
{
    type Item = Slice<'a, T>;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    const SECOND: u64 = 1_000_000_000;

    #[test]
    fn age_out() {
        let heatmap = Heatmap::<u64>::new(60 * SECOND, 2, SECOND, 5 * SECOND);
        heatmap.latch();
        assert_eq!(heatmap.total_count(), 0);
        heatmap.increment(time::precise_time_ns(), 1, 1);
        assert_eq!(heatmap.total_count(), 1);
        std::thread::sleep(std::time::Duration::new(5, 0));
        heatmap.latch();
        assert_eq!(heatmap.total_count(), 0);
    }

    #[test]
    fn out_of_bounds() {
        let heatmap = Heatmap::<u64>::new(60 * SECOND, 2, SECOND, 10 * SECOND);
        heatmap.latch();
        assert_eq!(heatmap.total_count(), 0);
        heatmap.increment(time::precise_time_ns() - 11 * SECOND, 1, 1);
        assert_eq!(heatmap.total_count(), 0);
        heatmap.increment(time::precise_time_ns() + 11 * SECOND, 1, 1);
        assert_eq!(heatmap.total_count(), 0);
    }

    #[test]
    fn threaded_access() {
        let heatmap = Arc::new(Heatmap::<u64>::new(SECOND, 2, SECOND, 60 * SECOND));
        let mut threads = Vec::new();

        for _ in 0..2 {
            let heatmap = heatmap.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1_000_000 {
                    heatmap.increment(time::precise_time_ns(), 1, 1);
                }
            }));
        }

        for thread in threads {
            thread.join().expect("Failed to join child thread");
        }

        std::thread::sleep(std::time::Duration::new(2, 0));

        assert_eq!(heatmap.total_count(), 2_000_000);
    }
}
