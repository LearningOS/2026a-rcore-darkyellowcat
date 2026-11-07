//! Per-process resource accounting and deadlock detection.

use alloc::{vec, vec::Vec};

/// Resource availability, allocations and pending requests for one resource group.
pub struct DeadlockDetector {
    available: Vec<usize>,
    allocation: Vec<Vec<usize>>,
    need: Vec<Vec<usize>>,
}

impl DeadlockDetector {
    /// Create an empty resource group for a new process.
    pub fn new() -> Self {
        Self {
            available: Vec::new(),
            allocation: Vec::new(),
            need: Vec::new(),
        }
    }

    /// Register one resource type and return its index.
    pub fn add_resource(&mut self, count: usize) -> usize {
        self.available.push(count);
        for row in &mut self.allocation {
            row.push(0);
        }
        for row in &mut self.need {
            row.push(0);
        }
        self.available.len() - 1
    }

    fn ensure_thread(&mut self, tid: usize) {
        while self.allocation.len() <= tid {
            self.allocation.push(vec![0; self.available.len()]);
            self.need.push(vec![0; self.available.len()]);
        }
    }

    /// Record one pending request before checking for deadlock.
    pub fn request(&mut self, tid: usize, resource: usize) {
        self.ensure_thread(tid);
        self.need[tid][resource] += 1;
    }

    /// Undo a rejected request without allocating or releasing a resource.
    pub fn cancel_request(&mut self, tid: usize, resource: usize) {
        self.need[tid][resource] -= 1;
    }

    /// Complete a pending request after the resource has been acquired.
    pub fn allocate(&mut self, tid: usize, resource: usize) {
        self.available[resource] -= 1;
        self.need[tid][resource] -= 1;
        self.allocation[tid][resource] += 1;
    }

    /// Return a held unit, or signal a semaphore without holding a unit.
    pub fn release(&mut self, tid: usize, resource: usize) {
        self.ensure_thread(tid);
        if self.allocation[tid][resource] > 0 {
            self.allocation[tid][resource] -= 1;
        }
        self.available[resource] += 1;
    }

    /// Simulate completion using current pending requests, not future claims.
    pub fn is_deadlocked(&self) -> bool {
        let mut work = self.available.clone();
        let mut finish = vec![false; self.allocation.len()];
        loop {
            let mut progress = false;
            for tid in 0..finish.len() {
                if !finish[tid]
                    && self.need[tid]
                        .iter()
                        .zip(&work)
                        .all(|(need, available)| need <= available)
                {
                    for (available, allocated) in work.iter_mut().zip(&self.allocation[tid]) {
                        *available += allocated;
                    }
                    finish[tid] = true;
                    progress = true;
                }
            }
            if !progress {
                return finish.iter().any(|finished| !finished);
            }
        }
    }
}
