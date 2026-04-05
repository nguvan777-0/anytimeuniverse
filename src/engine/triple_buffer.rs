use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::cell::UnsafeCell;

pub struct TripleBuffer<T> {
    buffers: [UnsafeCell<T>; 3],
    state: AtomicU8,
}

unsafe impl<T: Send> Send for TripleBuffer<T> {}
unsafe impl<T: Send> Sync for TripleBuffer<T> {}

impl<T: Default> TripleBuffer<T> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            buffers: core::array::from_fn(|_| UnsafeCell::new(T::default())),
            state: AtomicU8::new(0b00_10_01_00),
        })
    }

    pub fn write(&self) -> &mut T {
        let state = self.state.load(Ordering::Acquire);
        let back = (state >> 4) & 0b11;
        unsafe { &mut *self.buffers[back as usize].get() }
    }

    pub fn publish(&self) {
        let mut current = self.state.load(Ordering::Acquire);
        loop {
            let back = (current >> 4) & 0b11;
            let middle = (current >> 2) & 0b11;
            let front = current & 0b11;
            
            let new_state = (1 << 6) | (middle << 4) | (back << 2) | front;
            
            match self.state.compare_exchange_weak(
                current, new_state, Ordering::Release, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(val) => current = val,
            }
        }
    }

    pub fn update(&self) -> bool {
        let mut current = self.state.load(Ordering::Acquire);
        loop {
            let fresh = (current >> 6) & 1;
            if fresh == 0 {
                return false;
            }
            
            let back = (current >> 4) & 0b11;
            let middle = (current >> 2) & 0b11;
            let front = current & 0b11;
            
            let new_state = (0 << 6) | (back << 4) | (front << 2) | middle;
            
            match self.state.compare_exchange_weak(
                current, new_state, Ordering::Release, Ordering::Relaxed
            ) {
                Ok(_) => return true,
                Err(val) => current = val,
            }
        }
    }

    pub fn read(&self) -> &T {
        let state = self.state.load(Ordering::Acquire);
        let front = state & 0b11;
        unsafe { &*self.buffers[front as usize].get() }
    }
}
