use anyhow::{Error, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{error, info};

pub struct Worker<P, I, O> {
    name: String,
    processor: P,
    rx: Receiver<I>,
    tx: Sender<O>,
}

pub trait TaskProcessor<I, O> {
    fn process(&mut self, event: I) -> Result<O, Error>;

    fn init(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

impl<P, I, O> Worker<P, I, O>
where
    P: TaskProcessor<I, O> + Send + 'static,
    I: Send + 'static,
    O: Send + 'static,
{
    pub fn new(name: String, processor: P, rx: Receiver<I>, tx: Sender<O>) -> Self {
        Self {
            name,
            processor,
            rx,
            tx,
        }
    }

    pub fn start(mut self) -> JoinHandle<()> {
        let name = self.name.clone();

        let handle = tokio::task::spawn_blocking(move || {
            let ensure_clean_exit = AtomicBool::new(false);
            let name_guard = name.clone();
            let _guard = CallOnDrop::new(|| {
                if !ensure_clean_exit.load(Ordering::SeqCst) {
                    error!("Worker {} died unexpectedly! (Panic or Abort)", name_guard);
                } else {
                    info!("Worker {} stopped gracefully.", name_guard);
                }
            });

            // Initialize inside spawn_blocking so block_on can be used
            if let Err(e) = self.processor.init() {
                error!("Worker {} init failed: {:?}", name, e);
                return;
            }
            info!("Worker {} initialized successfully.", name);

            info!("Worker {} started.", name);

            while let Some(event) = self.rx.blocking_recv() {
                match self.processor.process(event) {
                    Ok(result) => {
                        if let Err(_) = self.tx.blocking_send(result) {
                            error!("Worker {} downstream closed, stopping.", name);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Worker {} process failed: {:?}", name, e);
                        continue;
                    }
                }
            }
            info!("Worker {} stopped.", name);
            ensure_clean_exit.store(true, Ordering::SeqCst);
        });

        handle
    }
}

struct CallOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> CallOnDrop<F> {
    fn new(f: F) -> Self {
        Self(Some(f))
    }
}
impl<F: FnOnce()> Drop for CallOnDrop<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}
