// Copyright 2020 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use core::time::Duration;

#[derive(Clone, Copy)]
pub enum Summary {
    Histogram(u64, u32, Option<Duration>),
}
