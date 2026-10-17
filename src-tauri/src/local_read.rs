//! Bounded disk/validation work for read-only native commands. A dropped IPC
//! future cannot release capacity while its blocking read is still running.

use tokio::sync::Semaphore;

static READS: Semaphore = Semaphore::const_new(2);
const WORKER_ERROR: &str = "The local read worker stopped. Saved data was not changed.";

pub(crate) async fn run<T: Send + 'static>(
    read: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    limited(&READS, read).await
}

async fn limited<T: Send + 'static>(
    slots: &'static Semaphore,
    read: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let permit = slots.acquire().await.map_err(|_| WORKER_ERROR.to_owned())?;
    tauri::async_runtime::spawn_blocking(move || {
        // Keep capacity owned by the actual worker, including after navigation
        // drops the awaiting command future. These jobs never connect an AI.
        let _permit = permit;
        read()
    })
    .await
    .map_err(|_| WORKER_ERROR.to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };

    #[test]
    fn disk_work_runs_off_the_calling_thread() {
        let caller = std::thread::current().id();
        let worker =
            tauri::async_runtime::block_on(run(|| Ok(std::thread::current().id()))).unwrap();
        assert_ne!(caller, worker);
    }

    #[test]
    fn a_read_error_is_preserved_without_retrying() {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let result = tauri::async_runtime::block_on(run::<()>(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            Err("The captured source changed.".into())
        }));
        assert_eq!(result.unwrap_err(), "The captured source changed.");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_panicked_worker_returns_only_a_fixed_error() {
        let result =
            tauri::async_runtime::block_on(run::<()>(|| panic!("private read diagnostic")));
        assert_eq!(result.unwrap_err(), WORKER_ERROR);
    }

    #[test]
    fn dropped_command_keeps_capacity_until_its_worker_really_finishes() {
        static TEST_SLOTS: Semaphore = Semaphore::const_new(1);
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_gate = Arc::clone(&gate);
        let (started, observed) = mpsc::channel();
        tauri::async_runtime::block_on(async move {
            let task = tokio::spawn(limited(&TEST_SLOTS, move || {
                started.send(()).unwrap();
                let (state, changed) = &*worker_gate;
                let ready = state.lock().unwrap();
                let (ready, _) = changed
                    .wait_timeout_while(ready, std::time::Duration::from_secs(5), |ready| !*ready)
                    .unwrap();
                if *ready {
                    Ok(())
                } else {
                    Err("The test gate timed out.".into())
                }
            }));
            observed
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
            let still_bounded = TEST_SLOTS.try_acquire().is_err();
            // Always unblock the worker, even if the capacity assertion fails.
            let (state, changed) = &*gate;
            *state.lock().unwrap() = true;
            changed.notify_one();
            let released = TEST_SLOTS.acquire().await.unwrap();
            drop(released);
            assert!(
                still_bounded,
                "navigation must not free a running read's slot"
            );
        });
    }
}
