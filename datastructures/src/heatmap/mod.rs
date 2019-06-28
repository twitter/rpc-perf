// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::counter::UnsignedCounterPrimitive;
use atomics::AtomicCounter;
use parking_lot::Mutex;
use std::convert::From;
use std::sync::Arc;

use crate::counter::Counter;
use crate::histogram::Histogram;

use time::Tm;

const SECOND: u64 = 1_000_000_000;

pub struct Slice<'a, T> {
    begin_utc: Tm,
    end_utc: Tm,
    begin_precise: u64,
    end_precise: u64,
    histogram: &'a Histogram<T>,
}

impl<'a, T> Slice<'a, T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive,
    u64: From<T>,
{
    pub fn begin_utc(&self) -> Tm {
        self.begin_utc
    }

    pub fn end_utc(&self) -> Tm {
        self.end_utc
    }

    pub fn begin_precise(&self) -> u64 {
        self.begin_precise
    }

    pub fn end_precise(&self) -> u64 {
        self.end_precise
    }

    pub fn histogram(&self) -> &Histogram<T> {
        &self.histogram
    }
}

pub struct Heatmap<T> {
    oldest_begin_precise: Counter<u64>, // this is the start time of oldest slice
    newest_begin_precise: Counter<u64>, // start time of newest slice
    newest_end_precise: Counter<u64>,   // end time of the oldest slice
    oldest_begin_utc: Arc<Mutex<Tm>>,   // relates start time of oldest slice to wall-clock
    resolution: Counter<u64>,           // number of NS per slice
    slices: Vec<Histogram<T>>,          // stores the `Histogram`s
    offset: Counter<usize>,             // indicates which `Histogram` is the oldest
}

impl<T> Heatmap<T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive,
    u64: From<T>,
{
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
            oldest_begin_precise: Counter::<u64>::new(adjusted_precise),
            newest_begin_precise: Counter::<u64>::new(adjusted_precise + span - resolution),
            newest_end_precise: Counter::<u64>::new(adjusted_precise + span),
            oldest_begin_utc: Arc::new(Mutex::new(adjusted_utc)),
            offset: Counter::<usize>::new(0),
            resolution: Counter::<u64>::new(resolution),
            slices,
        }
    }

    // internal function to calculate the index for a time
    fn get_index(&self, time: u64) -> Option<usize> {
        if self.oldest_begin_precise.get() < time && time < self.newest_end_precise.get() {
            let mut index = ((time - self.oldest_begin_precise.get()) / self.resolution.get())
                as usize
                + self.offset.get() as usize;
            if index >= self.slices.len() {
                index -= self.slices.len();
            }
            Some(index)
        } else {
            None
        }
    }

    // internal function to tick forward by one slice
    fn tick(&self) {
        self.slices[self.offset.get() as usize].clear();
        if self.offset.get() as usize == (self.slices.len() - 1) {
            self.offset.set(0);
        } else {
            self.offset.add(1);
        }
        self.oldest_begin_precise.add(self.resolution.get());
        self.newest_begin_precise.add(self.resolution.get());
        self.newest_end_precise.add(self.resolution.get());
        let current = *self.oldest_begin_utc.lock();
        *self.oldest_begin_utc.lock() =
            current + time::Duration::nanoseconds(self.resolution.get() as i64);
    }

    fn get_histogram(&self, index: usize) -> Option<&Histogram<T>> {
        if let Some(h) = self.slices.get(index) {
            Some(h)
        } else {
            None
        }
    }

    fn offset(&self) -> usize {
        self.offset.get() as usize
    }

    /// increment a time-value pair by count
    pub fn increment(&self, time: u64, value: u64, count: T) {
        if let Some(index) = self.get_index(time) {
            self.slices[index].increment(value, count);
        }
    }

    /// decrement a time-value pair by count
    pub fn decrement(&self, time: u64, value: u64, count: T) {
        if let Some(index) = self.get_index(time) {
            self.slices[index].decrement(value, count);
        }
    }

    /// moves the window forward as-needed, dropping older histograms
    pub fn latch(&self) {
        let time = time::precise_time_ns();

        // we only need to extend the Heatmap if we're currently writing to latest slice
        if time >= self.newest_begin_precise.get() {
            let ticks = (time - self.newest_begin_precise.get()) / self.resolution.get() + 1;
            for _ in 0..ticks {
                self.tick();
            }
        }
    }

    /// get the total number of samples stored in the heatmap
    pub fn total_count(&self) -> u64 {
        let mut count = 0;
        for histogram in &self.slices {
            count += histogram.total_count();
        }
        count
    }

    /// get the maximum number of samples in any time-value pair
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

    pub fn slices(&self) -> usize {
        self.slices.len()
    }

    pub fn buckets(&self) -> usize {
        self.slices[0].into_iter().count()
    }

    pub fn begin_utc(&self) -> Tm {
        *self.oldest_begin_utc.lock()
    }

    pub fn begin_precise(&self) -> u64 {
        self.oldest_begin_precise.get()
    }

    pub fn resolution(&self) -> u64 {
        self.resolution.get()
    }
}

pub struct Iter<'a, T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive,
    u64: From<T>,
{
    inner: &'a Heatmap<T>,
    index: usize,
}

impl<'a, T> Iter<'a, T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive,
    u64: From<T>,
{
    fn new(inner: &'a Heatmap<T>) -> Iter<'a, T> {
        Iter { inner, index: 0 }
    }
}

impl<'a, T> Iterator for Iter<'a, T>
where
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive,
    u64: From<T>,
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
    Box<AtomicCounter<Primitive = T>>: From<T>,
    T: UnsignedCounterPrimitive,
    u64: From<T>,
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
