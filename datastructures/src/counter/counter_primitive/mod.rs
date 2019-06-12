pub trait CounterPrimitive: PartialEq + Copy + Default {
    fn saturating_add(self, rhs: Self) -> Self;
    fn saturating_sub(self, rhs: Self) -> Self;
}

pub trait UnsignedCounterPrimitive: CounterPrimitive {}

impl CounterPrimitive for i8 {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl CounterPrimitive for i16 {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl CounterPrimitive for i32 {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl CounterPrimitive for i64 {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl CounterPrimitive for isize {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl CounterPrimitive for u8 {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl UnsignedCounterPrimitive for u8 {}

impl CounterPrimitive for u16 {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl UnsignedCounterPrimitive for u16 {}

impl CounterPrimitive for u32 {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl UnsignedCounterPrimitive for u32 {}

impl CounterPrimitive for u64 {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl UnsignedCounterPrimitive for u64 {}

impl CounterPrimitive for usize {
    fn saturating_add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
    fn saturating_sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl UnsignedCounterPrimitive for usize {}
