use crate::counter::*;
use atomics::*;

pub struct Buffer<T> {
    data: Vec<T>,
    read: AtomicUsize,
    write: AtomicUsize,
    len: AtomicUsize,
}

impl<T> Buffer<T>
where
    T: AtomicPrimitive + Default,
{
    /// Creates a new buffer which can hold `capacity` elements
    pub fn new(capacity: usize) -> Buffer<T> {
        let mut data = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            data.push(T::default())
        }
        data.shrink_to_fit();
        Buffer {
            data,
            read: AtomicUsize::new(0),
            write: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
        }
    }

    /// Clears the buffer of all contents
    pub fn clear(&self) {
        self.len.store(0, Ordering::SeqCst);
        self.read.store(0, Ordering::SeqCst);
        self.write.store(0, Ordering::SeqCst);
    }

    /// Tries to return one element from the buffer
    /// Returns Ok(None) if the buffer is empty
    /// Returns Ok(Some(T)) if we were able to read a value
    /// Returns Err(()) if there is a concurrent operation which interferes with read
    pub fn try_pop(&self) -> Result<Option<<T as AtomicPrimitive>::Primitive>, ()> {
        if self.len.load(Ordering::SeqCst) == 0 {
            Ok(None)
        } else {
            let current = self.read.load(Ordering::SeqCst);
            let new = if current + 1 >= self.data.len() {
                0
            } else {
                current + 1
            };
            if current == self.read.compare_and_swap(current, new, Ordering::SeqCst) {
                self.len.saturating_sub(1);
                Ok(Some(self.data[current].load(Ordering::SeqCst)))
            } else {
                Err(())
            }
        }
    }

    /// Loops try_pop() until it successfully returns an `Option<T>`
    pub fn pop(&self) -> Option<<T as AtomicPrimitive>::Primitive> {
        loop {
            if let Ok(result) = self.try_pop() {
                return result;
            }
        }
    }

    /// Tries to add one element to the buffer
    /// Returns Ok(()) if the element is added
    /// Returns Err(T) if there is a concurrent operation which interferes
    pub fn try_push(
        &self,
        value: <T as AtomicPrimitive>::Primitive,
    ) -> Result<(), <T as AtomicPrimitive>::Primitive> {
        while self.len.load(Ordering::SeqCst) >= self.data.len() {
            if self.try_pop().is_err() {
                return Err(value);
            }
        }
        let len = self.len.load(Ordering::SeqCst);
        let current = self.write.load(Ordering::SeqCst);
        let new = if current + 1 >= self.data.len() {
            0
        } else {
            current + 1
        };
        let previous = self.write.compare_and_swap(current, new, Ordering::SeqCst);
        if previous == current {
            let previous = self.len.compare_and_swap(len, len + 1, Ordering::SeqCst);
            if previous == len {
                self.data[current].swap(value, Ordering::SeqCst);
                Ok(())
            } else {
                Err(value)
            }
        } else {
            Err(value)
        }
    }

    /// Loops try_push() until success
    pub fn push(&self, value: <T as AtomicPrimitive>::Primitive) {
        let mut result = self.try_push(value);
        loop {
            if let Err(value) = result {
                result = self.try_push(value);
            } else {
                return;
            }
        }
    }

    /// Reads the contents of the buffer into a new Vec<T>
    pub fn as_vec(&self) -> Vec<<T as AtomicPrimitive>::Primitive> {
        let mut data = Vec::with_capacity(self.len.get());
        let mut read = self.read.get();
        for _ in 0..self.len.get() {
            data.push(self.data[read].load(Ordering::SeqCst));
            read += 1;
            if read == self.data.len() {
                read = 0;
            }
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let buffer = Buffer::<AtomicUsize>::new(4);
        for i in 1..=4 {
            assert!(buffer.try_push(i).is_ok());
        }
        for i in 1..=4 {
            assert_eq!(buffer.try_pop(), Ok(Some(i)));
        }
        assert_eq!(buffer.try_pop(), Ok(None));
    }

    #[test]
    fn blocking() {
        let buffer = Buffer::<AtomicUsize>::new(4);
        for i in 1..=4 {
            buffer.push(i);
        }
        for i in 1..=4 {
            assert_eq!(buffer.pop(), Some(i));
        }
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn wrapping() {
        let buffer = Buffer::<AtomicUsize>::new(4);
        for i in 1..=7 {
            assert!(buffer.try_push(i).is_ok());
        }
        for i in 4..=7 {
            assert_eq!(buffer.try_pop(), Ok(Some(i)));
        }
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn chasing() {
        let buffer = Buffer::<AtomicUsize>::new(4);
        for i in 1..=100 {
            assert!(buffer.try_push(i).is_ok());
            assert_eq!(buffer.try_pop(), Ok(Some(i)));
        }
        assert_eq!(buffer.pop(), None);
    }
}
