use anyhow::{Error, Result};
use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

#[async_trait]
pub trait Processor<I, O>: Send
where
    I: Send + 'static,
    O: Send + 'static,
{
    /// Process an input event and produce an output (Transformer mode)
    async fn process(self, rx: Receiver<I>, tx: Sender<O>) -> Result<JoinHandle<()>, Error>;
}

#[async_trait]
pub trait Producer<O>: Send
where
    O: Send + 'static,
{
    /// Produce an output (Source mode)
    async fn produce(self, tx: Sender<O>) -> Result<JoinHandle<()>, Error>;
}

#[async_trait]
pub trait Consumer<I>: Send
where
    I: Send + 'static,
{
    /// Consume an input event (Sink mode)
    async fn consume(self, rx: Receiver<I>) -> Result<JoinHandle<()>, Error>;
}
