use anyhow::{Error, Result};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

pub trait Processor<I, O>: Send
where
    I: Send + 'static,
    O: Send + 'static,
{
    /// Process an input event and produce an output (Transformer mode)
    fn process(self, rx: Receiver<I>, tx: Sender<O>) -> Result<JoinHandle<()>, Error>;
}

pub trait Producer<O>: Send
where
    O: Send + 'static,
{
    /// Produce an output (Source mode)
    fn produce(self, tx: Sender<O>) -> Result<JoinHandle<()>, Error>;
}

pub trait Consumer<I>: Send
where
    I: Send + 'static,
{
    /// Consume an input event (Sink mode)
    fn consume(self, rx: Receiver<I>) -> Result<JoinHandle<()>, Error>;
}
