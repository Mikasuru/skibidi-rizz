#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::cell::RefCell;

// for reusing allocations
pub struct ObjectPool<T> {
    objects: VecDeque<T>,
    create_fn: Box<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
    created: usize,
    hits: usize,
    misses: usize,
}

impl<T> ObjectPool<T> {
    // create a new object pool
    pub fn new<F>(create_fn: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            objects: VecDeque::with_capacity(max_size),
            create_fn: Box::new(create_fn),
            max_size,
            created: 0,
            hits: 0,
            misses: 0,
        }
    }

    // get an object from the pool or create a new one
    pub fn get(&mut self) -> T {
        if let Some(obj) = self.objects.pop_front() {
            self.hits += 1;
            obj
        } else {
            self.created += 1;
            self.misses += 1;
            (self.create_fn)()
        }
    }

    // return
    pub fn put(&mut self, obj: T) {
        if self.objects.len() < self.max_size {
            self.objects.push_back(obj);
        }
    }

    // pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            pool_size: self.objects.len(),
            max_size: self.max_size,
            created: self.created,
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    // clear
    pub fn clear(&mut self) {
        self.objects.clear();
    }
}

#[derive(Clone)]
pub struct SharedObjectPool<T> {
    pool: Arc<Mutex<ObjectPool<T>>>,
}

impl<T> SharedObjectPool<T> {
    // create a new shared object pool
    pub fn new<F>(create_fn: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            pool: Arc::new(Mutex::new(ObjectPool::new(create_fn, max_size))),
        }
    }

    // object from the pool
    pub fn get(&self) -> T {
        self.pool.lock().unwrap().get()
    }

    /// return an object to the pool
    pub fn put(&self, obj: T) {
        self.pool.lock().unwrap().put(obj);
    }

    // pool statistics
    pub fn stats(&self) -> PoolStats {
        self.pool.lock().unwrap().stats()
    }
}

// pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub pool_size: usize,
    pub max_size: usize,
    pub created: usize,
    pub hits: usize,
    pub misses: usize,
    pub hit_rate: f64,
}

// pooled buffer for packet data
pub struct PooledBuffer {
    data: Vec<u8>,
    pool: SharedObjectPool<Vec<u8>>,
}

impl PooledBuffer {
    // new pooled buffer
    pub fn new(capacity: usize, pool: &SharedObjectPool<Vec<u8>>) -> Self {
        let mut buffer = pool.get();
        buffer.clear();
        buffer.reserve(capacity);
        Self {
            data: buffer,
            pool: pool.clone(),
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn set_len(&mut self, len: usize) {
        self.data.resize(len, 0);
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        self.pool.put(std::mem::take(&mut self.data));
    }
}

// pooled string
pub struct PooledString {
    data: String,
    pool: SharedObjectPool<String>,
}

impl PooledString {
    pub fn new(pool: &SharedObjectPool<String>) -> Self {
        let mut string = pool.get();
        string.clear();
        Self {
            data: string,
            pool: pool.clone(),
        }
    }

    pub fn as_mut_string(&mut self) -> &mut String {
        &mut self.data
    }

    pub fn as_str(&self) -> &str {
        &self.data
    }
}

impl Drop for PooledString {
    fn drop(&mut self) {
        self.pool.put(std::mem::take(&mut self.data));
    }
}

// pool for timing
pub struct TimingPool {
    pool: SharedObjectPool<Instant>,
}

impl TimingPool {
    pub fn new(size: usize) -> Self {
        Self {
            pool: SharedObjectPool::new(Instant::now, size),
        }
    }

    pub fn get_time(&self) -> Instant {
        self.pool.get()
    }

    pub fn return_time(&self, _time: Instant) {
        // instants are not reusable
    }
}

// for better memory utilization
pub struct TieredBufferPool {
    small_buffers: SharedObjectPool<Vec<u8>>,
    medium_buffers: SharedObjectPool<Vec<u8>>,
    large_buffers: SharedObjectPool<Vec<u8>>,
    stats: Arc<Mutex<BufferPoolStats>>,
}

#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub small_hits: usize,
    pub medium_hits: usize,
    pub large_hits: usize,
    pub total_allocations: usize,
}

impl TieredBufferPool {
    // new tiered buffer pool
    pub fn new(small_size: usize, medium_size: usize, large_size: usize, pool_size: usize) -> Self {
        Self {
            small_buffers: SharedObjectPool::new(
                move || Vec::with_capacity(small_size),
                pool_size,
            ),
            medium_buffers: SharedObjectPool::new(
                move || Vec::with_capacity(medium_size),
                pool_size,
            ),
            large_buffers: SharedObjectPool::new(
                move || Vec::with_capacity(large_size),
                pool_size,
            ),
            stats: Arc::new(Mutex::new(BufferPoolStats {
                small_hits: 0,
                medium_hits: 0,
                large_hits: 0,
                total_allocations: 0,
            })),
        }
    }

    pub fn get_buffer(&self, size: usize) -> Vec<u8> {
        let mut stats = self.stats.lock().unwrap();
        stats.total_allocations += 1;

        if size <= 512 {
            stats.small_hits += 1;
            self.small_buffers.get()
        } else if size <= 2048 {
            stats.medium_hits += 1;
            self.medium_buffers.get()
        } else {
            stats.large_hits += 1;
            self.large_buffers.get()
        }
    }

    pub fn return_buffer(&self, mut buffer: Vec<u8>) {
        let capacity = buffer.capacity();
        buffer.clear();

        if capacity <= 512 {
            self.small_buffers.put(buffer);
        } else if capacity <= 2048 {
            self.medium_buffers.put(buffer);
        } else {
            self.large_buffers.put(buffer);
        }
    }

    pub fn get_stats(&self) -> BufferPoolStats {
        self.stats.lock().unwrap().clone()
    }
}

pub struct OptimizedBuffer {
    data: Vec<u8>,
    pool: Arc<TieredBufferPool>,
}

impl OptimizedBuffer {
    pub fn new(capacity: usize, pool: Arc<TieredBufferPool>) -> Self {
        let mut buffer = pool.get_buffer(capacity);
        buffer.clear();
        buffer.reserve(capacity);
        Self {
            data: buffer,
            pool,
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn set_len(&mut self, len: usize) {
        self.data.resize(len, 0);
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }
}

impl Drop for OptimizedBuffer {
    fn drop(&mut self) {
        self.pool.return_buffer(std::mem::take(&mut self.data));
    }
}

pub struct BufferArena {
    buffers: Vec<Vec<u8>>,
    index: RefCell<usize>,
    buffer_size: usize,
}

impl BufferArena {
    pub fn new(num_buffers: usize, buffer_size: usize) -> Self {
        let mut buffers = Vec::with_capacity(num_buffers);
        for _ in 0..num_buffers {
            buffers.push(Vec::with_capacity(buffer_size));
        }
        Self {
            buffers,
            index: RefCell::new(0),
            buffer_size,
        }
    }

    pub fn get_buffer(&mut self) -> Option<Vec<u8>> {
        let mut idx = self.index.borrow_mut();
        if *idx < self.buffers.len() {
            let mut buffer = std::mem::take(&mut self.buffers[*idx]);
            buffer.clear();
            *idx += 1;
            Some(buffer)
        } else {
            None
        }
    }

    pub fn return_buffer(&mut self, mut buffer: Vec<u8>) {
        if buffer.capacity() >= self.buffer_size {
            buffer.clear();
            let mut idx = self.index.borrow_mut();
            if *idx > 0 {
                *idx -= 1;
                self.buffers[*idx] = buffer;
            }
        }
    }

    pub fn reset(&mut self) {
        *self.index.borrow_mut() = 0;
    }
}
