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

use crate::client::{MICROSECOND, MILLISECOND, SECOND};
use crate::codec::Codec;
use crate::stats::Simple;
use crate::stats::Stat;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::thread_rng;

use bytes::BytesMut;
use mio::{Events, Poll, Token};
use ratelimiter::Ratelimiter;
use timer::Wheel;

use std::collections::VecDeque;

pub struct Common {
    id: usize,
    close_rate: Option<Ratelimiter>,
    connect_ratelimiter: Option<Ratelimiter>,
    connect_queue: VecDeque<Token>,
    connect_timeout: usize,
    events: Option<Events>,
    event_loop: Poll,
    codec: Box<Codec>,
    poolsize: usize,
    ready_queue: VecDeque<Token>,
    request_ratelimiter: Option<Ratelimiter>,
    request_timeout: usize,
    stats: Option<Simple>,
    timers: Wheel<Token>,
    last_timeouts: u64,
    tcp_nodelay: bool,
}

impl Common {
    pub fn new(id: usize, codec: Box<Codec>) -> Self {
        Self {
            id,
            close_rate: None,
            connect_ratelimiter: None,
            connect_queue: VecDeque::new(),
            connect_timeout: 200 * MILLISECOND / MICROSECOND,
            codec,
            events: None,
            event_loop: Poll::new().expect("Failed to create new event loop"),
            poolsize: 1,
            ready_queue: VecDeque::new(),
            request_ratelimiter: None,
            request_timeout: 200 * MILLISECOND / MICROSECOND,
            stats: None,
            timers: Wheel::<Token>::new(SECOND / MICROSECOND),
            last_timeouts: time::precise_time_ns(),
            tcp_nodelay: false,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn set_connect_ratelimit(&mut self, ratelimiter: Option<Ratelimiter>) {
        self.connect_ratelimiter = ratelimiter;
    }

    pub fn try_connect_wait(&self) -> Result<(), ()> {
        if let Some(ref ratelimiter) = self.connect_ratelimiter {
            ratelimiter.try_wait()
        } else {
            Ok(())
        }
    }

    pub fn set_connect_timeout(&mut self, microseconds: usize) {
        self.connect_timeout = microseconds;
    }

    pub fn connect_timeout(&self) -> usize {
        self.connect_timeout
    }

    pub fn set_request_timeout(&mut self, microseconds: usize) {
        self.request_timeout = microseconds;
    }

    pub fn request_timeout(&self) -> usize {
        self.request_timeout
    }

    pub fn set_request_ratelimit(&mut self, ratelimiter: Option<Ratelimiter>) {
        self.request_ratelimiter = ratelimiter;
    }

    pub fn try_request_wait(&self) -> Result<(), ()> {
        if let Some(ref ratelimiter) = self.request_ratelimiter {
            ratelimiter.try_wait()
        } else {
            Ok(())
        }
    }

    pub fn set_close_rate(&mut self, rate: Option<Ratelimiter>) {
        self.close_rate = rate;
    }

    pub fn should_close(&self) -> bool {
        if let Some(ref ratelimiter) = self.close_rate {
            ratelimiter.try_wait().map(|_| true).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn connect_pending(&self) -> usize {
        self.connect_queue.len()
    }

    pub fn connect_enqueue(&mut self, token: Token) {
        self.connect_queue.push_back(token);
    }

    pub fn connect_requeue(&mut self, token: Token) {
        self.connect_queue.push_front(token);
    }

    pub fn connect_dequeue(&mut self) -> Option<Token> {
        self.connect_queue.pop_front()
    }

    pub fn connect_shuffle(&mut self) {
        let mut tmp: Vec<Token> = self.connect_queue.drain(0..).collect();
        let mut rng = thread_rng();
        tmp.shuffle(&mut rng);
        for token in tmp {
            self.connect_queue.push_back(token);
        }
    }

    pub fn ready_enqueue(&mut self, token: Token) {
        self.ready_queue.push_back(token);
    }

    pub fn ready_requeue(&mut self, token: Token) {
        self.ready_queue.push_front(token);
    }

    pub fn ready_dequeue(&mut self) -> Option<Token> {
        self.ready_queue.pop_front()
    }

    pub fn set_stats(&mut self, recorder: Simple) {
        self.stats = Some(recorder);
    }

    pub fn stat_increment(&self, label: Stat) {
        if let Some(ref stats) = self.stats {
            stats.increment(label);
        }
    }

    pub fn stat_interval(&self, label: Stat, start: u64, stop: u64) {
        if let Some(ref stats) = self.stats {
            stats.time_interval(label, start, stop);
        }
    }

    pub fn heatmap_increment(&self, start: u64, stop: u64) {
        if let Some(ref stats) = self.stats {
            stats.heatmap_increment(start, stop);
        }
    }

    pub fn set_poolsize(&mut self, connections: usize) {
        self.poolsize = connections;
    }

    pub fn poolsize(&self) -> usize {
        self.poolsize
    }

    pub fn set_tcp_nodelay(&mut self, nodelay: bool) {
        self.tcp_nodelay = nodelay;
    }

    pub fn tcp_nodelay(&self) -> bool {
        self.tcp_nodelay
    }

    pub fn decode(&self, data: &[u8]) -> Result<codec::Response, codec::Error> {
        self.codec.decode(data)
    }

    pub fn encode(&mut self, buffer: &mut BytesMut, rng: &mut ThreadRng) {
        self.codec.encode(buffer, rng);
    }

    pub fn take_events(&mut self) -> Option<Events> {
        self.events.take()
    }

    pub fn set_events(&mut self, events: Option<Events>) {
        self.events = events;
    }

    pub fn event_loop(&self) -> &Poll {
        &self.event_loop
    }

    pub fn add_timer(&mut self, token: Token, microseconds: usize) {
        self.timers.add(token, microseconds);
    }

    pub fn cancel_timer(&mut self, token: Token) {
        self.timers.cancel(token);
    }

    pub fn get_timers(&mut self) -> Vec<Token> {
        let now = time::precise_time_ns();
        let last = self.last_timeouts;
        let ticks = (now - last) as usize / MICROSECOND;
        self.last_timeouts = now;
        self.timers.tick(ticks)
    }
}
