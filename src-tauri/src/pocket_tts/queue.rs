//! One section owns the worker until it has stopped and been reaped. At most
//! four other reviewed sections may wait; queued cancellation starts no work.

use super::{cancelled_failure, failure};
use crate::provider::{ProviderFailure, ProviderFailureKind};
use tokio::sync::{Semaphore, SemaphorePermit, oneshot};

pub(super) struct WorkerQueue {
    active: Semaphore,
    capacity: Semaphore,
}

pub(super) struct WorkerPermit<'a> {
    _active: SemaphorePermit<'a>,
    _capacity: SemaphorePermit<'a>,
}

impl WorkerQueue {
    pub(super) const fn new() -> Self {
        Self {
            active: Semaphore::const_new(1),
            capacity: Semaphore::const_new(5),
        }
    }

    pub(super) fn close(&self) {
        self.active.close();
        self.capacity.close();
    }

    pub(super) async fn acquire<'a>(
        &'a self,
        cancellation: &mut oneshot::Receiver<()>,
    ) -> Result<WorkerPermit<'a>, ProviderFailure> {
        if !matches!(
            cancellation.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ) {
            return Err(cancelled_failure());
        }
        let capacity = self.capacity.try_acquire().map_err(|error| match error {
            tokio::sync::TryAcquireError::Closed => cancelled_failure(),
            tokio::sync::TryAcquireError::NoPermits => failure(
                ProviderFailureKind::ReviewUnavailable,
                "The local narration queue is full. Wait for narration to finish or cancel a request.",
            ),
        })?;
        let active = tokio::select! {
            biased;
            _ = &mut *cancellation => return Err(cancelled_failure()),
            permit = self.active.acquire() => permit.map_err(|_| cancelled_failure())?,
        };
        if !matches!(
            cancellation.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ) {
            return Err(cancelled_failure());
        }
        Ok(WorkerPermit {
            _active: active,
            _capacity: capacity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, time::Duration};

    async fn wait_capacity(queue: &WorkerQueue, expected: usize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while queue.capacity.available_permits() != expected {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn queued_section_waits_for_entire_owner_and_cancel_frees_capacity() {
        let queue = Arc::new(WorkerQueue::new());
        let (_first_tx, mut first_rx) = oneshot::channel();
        let first = queue.acquire(&mut first_rx).await.unwrap();
        let (cancel, mut receiver) = oneshot::channel();
        let pending = Arc::clone(&queue);
        let next = tokio::spawn(async move { pending.acquire(&mut receiver).await.map(|_| ()) });
        wait_capacity(&queue, 3).await;
        assert!(!next.is_finished());
        cancel.send(()).unwrap();
        assert_eq!(
            next.await.unwrap().unwrap_err().kind,
            ProviderFailureKind::RequestCancelled
        );
        assert_eq!(queue.capacity.available_permits(), 4);
        assert_eq!(queue.active.available_permits(), 0);
        drop(first);
        assert_eq!(queue.capacity.available_permits(), 5);
    }

    #[tokio::test]
    async fn whole_section_handoff_and_four_waiter_limit_are_bounded() {
        let queue = Arc::new(WorkerQueue::new());
        let (_first_tx, mut first_rx) = oneshot::channel();
        let first = queue.acquire(&mut first_rx).await.unwrap();
        let mut waiting = vec![];
        let mut cancellation = vec![];
        for expected in (0..4).rev() {
            let (sender, mut receiver) = oneshot::channel();
            cancellation.push(sender);
            let next = Arc::clone(&queue);
            waiting.push(tokio::spawn(async move {
                next.acquire(&mut receiver).await.map(|_| ())
            }));
            wait_capacity(&queue, expected).await;
        }
        let (_overflow_tx, mut overflow_rx) = oneshot::channel();
        assert_eq!(
            queue.acquire(&mut overflow_rx).await.err().unwrap().kind,
            ProviderFailureKind::ReviewUnavailable
        );
        assert!(waiting.iter().all(|task| !task.is_finished()));
        drop(first);
        for task in waiting {
            task.await.unwrap().unwrap();
        }
        assert_eq!(queue.capacity.available_permits(), 5);
    }

    #[tokio::test]
    async fn dropped_waiter_and_shutdown_release_slots_without_starting_work() {
        let queue = Arc::new(WorkerQueue::new());
        let (_first_tx, mut first_rx) = oneshot::channel();
        let _first = queue.acquire(&mut first_rx).await.unwrap();
        let (_sender, mut receiver) = oneshot::channel();
        let next = Arc::clone(&queue);
        let task = tokio::spawn(async move { next.acquire(&mut receiver).await.map(|_| ()) });
        wait_capacity(&queue, 3).await;
        task.abort();
        let _ = task.await;
        wait_capacity(&queue, 4).await;
        let (_sender, mut receiver) = oneshot::channel();
        let next = Arc::clone(&queue);
        let task = tokio::spawn(async move { next.acquire(&mut receiver).await.map(|_| ()) });
        wait_capacity(&queue, 3).await;
        queue.close();
        assert_eq!(
            task.await.unwrap().unwrap_err().kind,
            ProviderFailureKind::RequestCancelled
        );
        wait_capacity(&queue, 4).await;
        let (_sender, mut receiver) = oneshot::channel();
        assert_eq!(
            queue.acquire(&mut receiver).await.err().unwrap().kind,
            ProviderFailureKind::RequestCancelled
        );
    }
}
