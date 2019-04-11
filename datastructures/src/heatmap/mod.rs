//  Copyright 2019 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use crate::counter::Counter;
use crate::histogram::{Histogram, LatchedHistogram};
use crate::wrapper::RwWrapper;

use time::Tm;

const SECOND: u64 = 1_000_000_000;

pub struct Slice {
    begin_utc: Tm,
    end_utc: Tm,
    begin_precise: usize,
    end_precise: usize,
    histogram: LatchedHistogram,
}

impl Slice {
    pub fn begin_utc(&self) -> Tm {
        self.begin_utc
    }

    pub fn end_utc(&self) -> Tm {
        self.end_utc
    }

    pub fn begin_precise(&self) -> usize {
        self.begin_precise
    }

    pub fn end_precise(&self) -> usize {
        self.end_precise
    }

    pub fn histogram(&self) -> &LatchedHistogram {
        &self.histogram
    }
}

pub struct Builder {
    min: usize,
    max: usize,
    precision: usize,
    resolution: usize,
    span: usize,
}

impl Builder {
    pub fn new(min: usize, max: usize, precision: usize, resolution: usize, span: usize) -> Self {
        Self {
            min,
            max,
            precision,
            resolution,
            span,
        }
    }

    pub fn build(&self) -> Heatmap {
        self::Heatmap::new(self.max, self.precision, self.resolution, self.span)
    }
}

#[derive(Clone)]
pub struct Heatmap {
    oldest_begin_precise: Counter, // this is the start time of oldest slice
    newest_begin_precise: Counter, // start time of newest slice
    newest_end_precise: Counter,   // end time of the oldest slice
    oldest_begin_utc: RwWrapper<Tm>, // relates start time of oldest slice to wall-clock
    resolution: Counter,           // number of NS per slice
    slices: Vec<LatchedHistogram>, // stores the `Histogram`s
    offset: Counter,               // indicates which `Histogram` is the oldest
}

impl Heatmap {
    pub fn new(max: usize, precision: usize, resolution: usize, span: usize) -> Self {
        // build the Histograms
        let num_slices = span / resolution;
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            slices.push(LatchedHistogram::new(max, precision));
        }

        // get time and align with previous top of minute
        let now_utc = time::now_utc();
        let now_precise = time::precise_time_ns();
        let adjusted_precise =
            now_precise - now_utc.tm_nsec as u64 - now_utc.tm_sec as u64 * SECOND; // set backward to top of minute
        let adjusted_utc =
            now_utc - time::Duration::nanoseconds((now_precise - adjusted_precise) as i64); // set backward to top of minute

        Heatmap {
            oldest_begin_precise: Counter::new(adjusted_precise as usize),
            newest_begin_precise: Counter::new(adjusted_precise as usize + span - resolution),
            newest_end_precise: Counter::new(adjusted_precise as usize + span),
            oldest_begin_utc: RwWrapper::new(adjusted_utc),
            offset: Counter::new(0),
            resolution: Counter::new(resolution),
            slices,
        }
    }

    // internal function to calculate the index for a time
    fn get_index(&self, time: usize) -> Option<usize> {
        if self.oldest_begin_precise.get() < time && time < self.newest_end_precise.get() {
            let mut index = ((time - self.oldest_begin_precise.get()) / self.resolution.get())
                as usize
                + self.offset.get();
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
        self.slices[self.offset.get()].clear();
        if self.offset.get() == (self.slices.len() - 1) {
            self.offset.set(0);
        } else {
            self.offset.incr(1);
        }
        self.oldest_begin_precise.incr(self.resolution.get());
        self.newest_begin_precise.incr(self.resolution.get());
        self.newest_end_precise.incr(self.resolution.get());
        unsafe {
            (*self.oldest_begin_utc.lock()) = (*self.oldest_begin_utc.get())
                + time::Duration::nanoseconds(self.resolution.get() as i64);
        }
    }

    fn get_histogram(&self, index: usize) -> Option<LatchedHistogram> {
        if let Some(h) = self.slices.get(index) {
            Some(h.clone())
        } else {
            None
        }
    }

    fn offset(&self) -> usize {
        self.offset.get()
    }

    /// increment a time-value pair by count
    pub fn incr(&self, time: usize, value: usize, count: usize) {
        if let Some(index) = self.get_index(time) {
            self.slices[index].incr(value, count);
        }
    }

    /// decrement a time-value pair by count
    pub fn decr(&self, time: usize, value: usize, count: usize) {
        if let Some(index) = self.get_index(time) {
            self.slices[index].decr(value, count);
        }
    }

    /// moves the window forward as-needed, dropping older histograms
    pub fn latch(&self) {
        let time = time::precise_time_ns() as usize;

        // we only need to extend the Heatmap if we're currently writing to latest slice
        if time >= self.newest_begin_precise.get() {
            let ticks = (time - self.newest_begin_precise.get()) / self.resolution.get() + 1;
            for _ in 0..ticks {
                self.tick();
            }
        }
    }

    /// get the total number of samples stored in the heatmap
    pub fn samples(&self) -> usize {
        let mut count = 0;
        for histogram in &self.slices {
            count += histogram.samples();
        }
        count
    }

    /// get the maximum number of samples in any time-value pair
    pub fn highest_count(&self) -> usize {
        let mut count = 0;
        for histogram in &self.slices {
            let c = histogram.highest_count();
            if c > count {
                count = c;
            }
        }
        count
    }

    pub fn slices(&self) -> usize {
        self.slices.len()
    }

    pub fn buckets(&self) -> usize {
        self.slices[0].buckets()
    }

    pub fn begin_utc(&self) -> Tm {
        unsafe { (*self.oldest_begin_utc.get()) }
    }

    pub fn begin_precise(&self) -> usize {
        self.oldest_begin_precise.get()
    }

    pub fn resolution(&self) -> usize {
        self.resolution.get()
    }
}

pub struct Iter<'a> {
    inner: &'a Heatmap,
    index: usize,
}

impl<'a> Iter<'a> {
    fn new(inner: &'a Heatmap) -> Iter<'a> {
        Iter { inner, index: 0 }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Slice;

    fn next(&mut self) -> Option<Slice> {
        if self.index >= self.inner.slices() {
            None
        } else {
            let mut index = self.index + self.inner.offset();
            if index >= self.inner.slices() {
                index -= self.inner.slices();
            }
            let heatmap_begin_precise = self.inner.begin_precise();
            let begin_precise = heatmap_begin_precise + self.index * self.inner.resolution();
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

impl<'a> IntoIterator for &'a Heatmap {
    type Item = Slice;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    const SECOND: usize = 1_000_000_000;

    #[test]
    fn age_out() {
        let heatmap = Heatmap::new(60 * SECOND, 2, SECOND, 5 * SECOND);
        heatmap.latch();
        assert_eq!(heatmap.samples(), 0);
        heatmap.incr(time::precise_time_ns() as usize, 1, 1);
        assert_eq!(heatmap.samples(), 1);
        std::thread::sleep(std::time::Duration::new(5, 0));
        heatmap.latch();
        assert_eq!(heatmap.samples(), 0);
    }

    #[test]
    fn out_of_bounds() {
        let heatmap = Heatmap::new(60 * SECOND, 2, SECOND, 10 * SECOND);
        heatmap.latch();
        assert_eq!(heatmap.samples(), 0);
        heatmap.incr(time::precise_time_ns() as usize - 11 * SECOND, 1, 1);
        assert_eq!(heatmap.samples(), 0);
        heatmap.incr(time::precise_time_ns() as usize + 11 * SECOND, 1, 1);
        assert_eq!(heatmap.samples(), 0);
    }

    #[test]
    fn threaded_access() {
        let heatmap = Heatmap::new(SECOND, 2, SECOND, 60 * SECOND);
        let mut threads = Vec::new();

        for _ in 0..2 {
            let heatmap = heatmap.clone();
            threads.push(thread::spawn(move || {
                for _ in 0..1_000_000 {
                    heatmap.incr(time::precise_time_ns() as usize, 1, 1);
                }
            }));
        }

        for thread in threads {
            thread.join().expect("Failed to join child thread");
        }

        std::thread::sleep(std::time::Duration::new(1, 0));

        assert_eq!(heatmap.samples(), 2_000_000);
    }
}
