// Copyright 2019 Twitter, Inc.
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use crate::{AtomicCounter, AtomicI64, Ordering};

impl AtomicCounter for AtomicI64 {
    fn fetch_add(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_add(value, order)
    }

    fn fetch_sub(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_sub(value, order)
    }

    fn fetch_and(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_and(value, order)
    }

    fn fetch_nand(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_nand(value, order)
    }

    fn fetch_or(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_or(value, order)
    }

    fn fetch_xor(&self, value: Self::Primitive, order: Ordering) -> Self::Primitive {
        self.inner.fetch_xor(value, order)
    }
}
