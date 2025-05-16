use std::{sync::Arc, thread, time::Duration};

use flume::{Receiver, Sender};
use sp1_sdk::{CudaProver, ProverClient};
use tokio::sync::Mutex;

use crate::{cache::KeysCache, types::Task, worker::Worker};

pub struct Pool {
    tx: Sender<Task>,
    rx: Receiver<Task>,
    ack_tx: Sender<()>,
    ack: Receiver<()>,
    min_workers: usize,
    max_workers: usize,
    kill_pending: usize,
    target_queue_size: usize,
    gradient: f64,
    frequency: Duration,
    cache: KeysCache,
    sp1gpu: Option<Arc<Mutex<CudaProver>>>,
}

impl Pool {
    pub fn new(cache: usize, gpu: bool) -> Self {
        let (tx, rx) = flume::unbounded();
        let (ack_tx, ack) = flume::unbounded();
        let cache = KeysCache::new(cache);
        let sp1gpu = gpu.then(|| {
            tracing::info!("initializing GPU support...");

           let client =  ProverClient::builder().cuda().build();
           let client = Mutex::new(client);
           let client = Arc::new(client);

           client
        });

        Self {
            tx,
            rx,
            ack_tx,
            ack,
            min_workers: 1,
            max_workers: 8,
            kill_pending: 0,
            target_queue_size: 2,
            gradient: 0.1,
            frequency: Duration::from_secs(600),
            cache,
            sp1gpu,
        }
    }

    pub fn with_min_workers(mut self, min_workers: usize) -> Self {
        self.min_workers = min_workers;
        self
    }

    pub fn with_max_workers(mut self, max_workers: usize) -> Self {
        self.max_workers = max_workers;
        self
    }

    pub fn with_target_queue_size(mut self, target_queue_size: usize) -> Self {
        self.target_queue_size = target_queue_size;
        self
    }

    pub fn with_gradient(mut self, gradient: f64) -> Self {
        self.gradient = gradient;
        self
    }

    pub fn with_frequency(mut self, frequency_secs: u64) -> Self {
        self.frequency = Duration::from_secs(frequency_secs);
        self
    }

    pub fn run(mut self) -> Sender<Task> {
        let tx = self.tx.clone();

        self.scale();

        thread::spawn(move || {
            thread::sleep(self.frequency);

            self.scale();
        });

        tx
    }

    pub fn workers(&self) -> usize {
        self.tx.receiver_count().saturating_sub(1)
    }

    pub fn queued(&self) -> usize {
        self.tx.len()
    }

    pub fn scale(&mut self) {
        tracing::debug!("scaling workers...");

        let error = self.queued() as isize - self.target_queue_size as isize;
        let delta = (self.gradient * error as f64).round() as isize;

        for _ack in self.ack.try_iter() {
            tracing::debug!("pool ack kill received");

            self.kill_pending = self.kill_pending.saturating_sub(1);
        }

        let workers = self.workers();

        if workers < self.min_workers {
            let added = self.min_workers - workers;

            for _ in 0..added {
                tracing::debug!("spawn worker; workers: {workers}, queue: {}", self.queued());
                self.spawn_worker();
            }
        } else if delta > 0 {
            let new_workers = workers + delta as usize;
            let new_workers = new_workers.min(self.max_workers);
            let added = new_workers.saturating_sub(workers);

            for _ in 0..added {
                tracing::debug!("spawn worker; workers: {workers}, queue: {}", self.queued());
                self.spawn_worker();
            }
        } else if delta < 0 {
            let new_workers = workers as isize + delta;
            let new_workers = new_workers.max(self.min_workers as isize) as usize;
            let removed = workers
                .saturating_sub(new_workers)
                .saturating_sub(self.kill_pending);

            for _ in 0..removed {
                tracing::debug!("kill worker; workers: {workers}, queue: {}", self.queued());
                self.ack_tx.send(()).ok();
                self.kill_pending = self.kill_pending.saturating_add(1);
            }
        }

        tracing::info!(
            "prover scaling completed; workers {}, queued {}",
            self.workers(),
            self.queued()
        );
    }

    pub fn spawn_worker(&self) {
        let cache = self.cache.clone();
        let rx = self.rx.clone();
        let tx = self.ack_tx.clone();

        Worker::spawn(cache, rx, tx, self.sp1gpu.clone());
    }
}
