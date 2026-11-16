//! Helper threads for the real-time graph (NFR-1, A-024): the independent nodes of one dependency level are shared
//! between the audio thread and a few helpers. No allocation and no lock on the audio thread:
//!
//! - the audio thread writes the job (pointers to the level's nodes), then publishes one atomic word that carries
//!   the job's epoch (high 32 bits) and the next node index (low 32 bits);
//! - every participant claims nodes with a compare-and-swap on that word, so a helper still looking at an old epoch
//!   can never claim a node of the new one;
//! - the audio thread works too, then spins until every claimed node is done;
//! - helpers spin briefly after each job, then park; the audio thread wakes them with `unpark` (a non-blocking
//!   signal).
//!
//! Nodes of a level are disjoint and only read nodes of earlier levels (sidechain keys), so the work is race-free;
//! the contributions to destinations are added afterwards, serially, in a fixed order: results are identical to the
//! single-threaded path.

use std::cell::UnsafeCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::{JoinHandle, Thread};
use std::time::{Duration, Instant};

/// A node processing job: `run(index)` processes the node at that index of the current level.
pub trait LevelJob: Sync {
    fn run(&self, index: usize);
}

struct Job {
    ptr: *const (dyn LevelJob + 'static),
    len: usize,
}

struct Shared {
    /// epoch << 32 | next index.
    word: AtomicU64,
    done: AtomicUsize,
    job: UnsafeCell<Job>,
    stop: AtomicBool,
}

// SAFETY: `job` is written only by the audio thread before it publishes a new epoch with Release ordering, and read
// by helpers only after they observed that epoch with Acquire ordering and claimed an index of it.
unsafe impl Sync for Shared {}
unsafe impl Send for Shared {}

pub struct Pool {
    shared: Arc<Shared>,
    threads: Vec<Thread>,
    handles: Vec<JoinHandle<()>>,
    epoch: std::cell::Cell<u32>,
}

// SAFETY: `epoch` is only touched by the single thread that calls `run_level` (the audio thread).
unsafe impl Sync for Pool {}

fn claim(shared: &Shared, epoch: u32, len: usize) -> Option<usize> {
    loop {
        let v = shared.word.load(Ordering::Acquire);
        if (v >> 32) as u32 != epoch {
            return None;
        }
        let i = (v & 0xFFFF_FFFF) as usize;
        if i >= len {
            return None;
        }
        if shared.word.compare_exchange_weak(v, v + 1, Ordering::AcqRel, Ordering::Acquire).is_ok() {
            return Some(i);
        }
    }
}

fn helper(shared: Arc<Shared>) {
    let mut seen = 0u32;
    loop {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        let epoch = (shared.word.load(Ordering::Acquire) >> 32) as u32;
        if epoch != seen {
            seen = epoch;
            // SAFETY: the job of this epoch was written before the epoch was published.
            let (ptr, len) = unsafe {
                let j = &*shared.job.get();
                (j.ptr, j.len)
            };
            while let Some(i) = claim(&shared, epoch, len) {
                // SAFETY: the job outlives the level: the audio thread waits for every claimed index.
                unsafe { (*ptr).run(i) };
                shared.done.fetch_add(1, Ordering::Release);
            }
            continue;
        }
        // Spin a little (levels follow each other within microseconds), then park.
        let t = Instant::now();
        while (shared.word.load(Ordering::Acquire) >> 32) as u32 == seen && t.elapsed() < Duration::from_micros(200) {
            std::hint::spin_loop();
        }
        if (shared.word.load(Ordering::Acquire) >> 32) as u32 == seen {
            std::thread::park_timeout(Duration::from_millis(20));
        }
    }
}

impl Pool {
    pub fn new(helpers: usize) -> Arc<Pool> {
        let shared = Arc::new(Shared {
            word: AtomicU64::new(0),
            done: AtomicUsize::new(0),
            job: UnsafeCell::new(Job { ptr: &NoJob as *const dyn LevelJob, len: 0 }),
            stop: AtomicBool::new(false),
        });
        let mut handles = Vec::new();
        for k in 0..helpers {
            let s = shared.clone();
            if let Ok(h) = std::thread::Builder::new().name(format!("stemwise-dsp-{k}")).spawn(move || helper(s)) {
                handles.push(h);
            }
        }
        let threads = handles.iter().map(|h| h.thread().clone()).collect();
        Arc::new(Pool { shared, threads, handles, epoch: std::cell::Cell::new(0) })
    }

    /// Helpers for this machine: up to 3, leaving cores for the interface and the reader.
    pub fn default_helpers() -> usize {
        std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(2)).clamp(0, 3)
    }

    pub fn helpers(&self) -> usize {
        self.handles.len()
    }

    /// Runs `job.run(i)` for `i in 0..len` on this thread and the helpers; returns when all are done.
    pub fn run_level(&self, job: &dyn LevelJob, len: usize) {
        if len == 0 {
            return;
        }
        let epoch = self.epoch.get().wrapping_add(1).max(1);
        self.epoch.set(epoch);
        // SAFETY: no helper reads `job` before the new epoch is published below; the reference is only used until
        // this function returns, after every claimed index is done.
        unsafe {
            let ptr: *const (dyn LevelJob + '_) = job;
            *self.shared.job.get() = Job {
                ptr: std::mem::transmute::<*const (dyn LevelJob + '_), *const (dyn LevelJob + 'static)>(ptr),
                len,
            };
        }
        self.shared.done.store(0, Ordering::Relaxed);
        self.shared.word.store(u64::from(epoch) << 32, Ordering::Release);
        for t in &self.threads {
            t.unpark();
        }
        let mut mine = 0;
        while let Some(i) = claim(&self.shared, epoch, len) {
            job.run(i);
            mine += 1;
        }
        // Wait for the nodes the helpers claimed.
        while self.shared.done.load(Ordering::Acquire) + mine < len {
            std::hint::spin_loop();
        }
    }
}

struct NoJob;
impl LevelJob for NoJob {
    fn run(&self, _: usize) {}
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        for t in &self.threads {
            t.unpark();
        }
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    struct Count(Vec<AtomicU32>);
    impl LevelJob for Count {
        fn run(&self, i: usize) {
            self.0[i].fetch_add(1, Ordering::Relaxed);
        }
    }

    /// req: NFR-1
    #[test]
    fn every_node_runs_exactly_once_per_level_across_many_levels() {
        let pool = Pool::new(3);
        let job = Count((0..40).map(|_| AtomicU32::new(0)).collect());
        for _ in 0..20_000 {
            pool.run_level(&job, 40);
        }
        assert!(job.0.iter().all(|c| c.load(Ordering::Relaxed) == 20_000));
    }
}
