// Copyright 2022 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use rustcommon_time::Nanoseconds;

pub type Duration = rustcommon_time::Duration<Nanoseconds<u64>>;
pub type Instant = rustcommon_time::Instant<Nanoseconds<u64>>;
