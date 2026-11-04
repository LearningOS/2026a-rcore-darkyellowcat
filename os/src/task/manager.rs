//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use core::cmp::Ordering;

const BIG_STRIDE: usize = 10_000;

/// Compare two strides, return Ordering::Less if a < b, Ordering::Greater if a > b, and Ordering::Equal if a == bf
fn stride_cmp(a: usize, b: usize) -> Ordering {
    let diff = a.wrapping_sub(b);
    let half = usize::MAX / 2 + 1;

    if diff == 0 || diff == half {
        Ordering::Equal
    } else if diff > half {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let index = self.ready_queue
            .iter()
            .enumerate()
            .min_by(|(_, task_a), (_, task_b)| {
                let stride_a = task_a.inner_exclusive_access().stride;
                let stride_b = task_b.inner_exclusive_access().stride;
                stride_cmp(stride_a, stride_b)
            })
            .map(|(i, _)| i)?;

        let task = self.ready_queue.remove(index)?;

        {
            let mut inner = task.inner_exclusive_access();

            let pass = BIG_STRIDE / inner.priority;

            inner.stride = inner.stride.wrapping_add(pass);
        }

        Some(task)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
