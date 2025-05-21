use std::{thread, time::Duration};

use flume::{Receiver, Sender};
use serde_json::{json, Value};
use valence_coprocessor::Hash;

use crate::{Historical, ServiceVm, ServiceZkVm};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    Prove {
        program: Hash,
        args: Value,
        payload: Option<Value>,
    },
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ack {
    Kill,
}

pub struct Pool {
    tx: Sender<Job>,
    rx: Receiver<Job>,
    ack_tx: Sender<Ack>,
    ack: Receiver<Ack>,
    min_workers: usize,
    max_workers: usize,
    kill_pending: usize,
    target_queue_size: usize,
    gradient: f64,
    frequency: Duration,
    historical: Historical,
    vm: ServiceVm,
    zkvm: ServiceZkVm,
}

impl Pool {
    pub fn new(historical: Historical, vm: ServiceVm, zkvm: ServiceZkVm) -> Self {
        let (tx, rx) = flume::unbounded();
        let (ack_tx, ack) = flume::unbounded();

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
            historical,
            vm,
            zkvm,
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

    pub fn run(mut self) -> Sender<Job> {
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

        for ack in self.ack.try_iter() {
            tracing::debug!("pool ack received: {ack:?}");

            match ack {
                Ack::Kill => self.kill_pending = self.kill_pending.saturating_sub(1),
            }
        }

        let workers = self.workers();

        if workers < self.min_workers {
            let added = self.min_workers - workers;

            for _ in 0..added {
                tracing::debug!("spawn worker; workers: {workers}, queue: {}", self.queued());
                self.new_worker().spawn();
            }
        } else if delta > 0 {
            let new_workers = workers + delta as usize;
            let new_workers = new_workers.min(self.max_workers);
            let added = new_workers.saturating_sub(workers);

            for _ in 0..added {
                tracing::debug!("spawn worker; workers: {workers}, queue: {}", self.queued());
                self.new_worker().spawn();
            }
        } else if delta < 0 {
            let new_workers = workers as isize + delta;
            let new_workers = new_workers.max(self.min_workers as isize) as usize;
            let removed = workers
                .saturating_sub(new_workers)
                .saturating_sub(self.kill_pending);

            for _ in 0..removed {
                tracing::debug!("kill worker; workers: {workers}, queue: {}", self.queued());
                self.ack_tx.send(Ack::Kill).ok();
                self.kill_pending = self.kill_pending.saturating_add(1);
            }
        }

        tracing::debug!("scaling completed.");
    }

    pub fn new_worker(&self) -> Worker {
        Worker {
            historical: self.historical.clone(),
            vm: self.vm.clone(),
            zkvm: self.zkvm.clone(),
            rx: self.rx.clone(),
            tx: self.ack_tx.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Worker {
    historical: Historical,
    vm: ServiceVm,
    zkvm: ServiceZkVm,
    rx: Receiver<Job>,
    tx: Sender<Ack>,
}

impl Worker {
    pub fn prove(&self, program: Hash, args: Value, payload: Option<Value>) {
        tracing::debug!("worker recv: {}", hex::encode(program));

        let ctx = self.historical.context(program);
        let res = ctx.get_proof(&self.vm, &self.zkvm, args.clone());

        tracing::debug!(
            "worker received proof: {}, {}",
            hex::encode(program),
            res.is_ok()
        );

        let log = ctx.get_log().unwrap_or_default();
        let mut args = json!({
            "success": res.is_ok(),
            "args": args,
            "log": log,
            "payload": payload,
        });

        match res {
            Ok(p) => args["proof"] = p.to_base64().into(),
            Err(e) => tracing::debug!("error on computed proof: {e}"),
        }

        if ctx.entrypoint(&self.vm, args.clone()).is_err() {
            tracing::debug!(
                "failed to call library entrypoint for program `{}` with args `{args:?}`",
                hex::encode(program)
            );
        }
    }

    pub fn spawn(self) {
        thread::spawn(move || {
            while let Ok(j) = self.rx.recv() {
                match j {
                    Job::Prove {
                        program,
                        args,
                        payload,
                    } => self.prove(program, args, payload),
                    Job::Quit => {
                        self.tx.send(Ack::Kill).ok();
                        break;
                    }
                }
            }
        });
    }
}
