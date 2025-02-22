use super::{Write, S};
use crate::{ActorError, Result, Who};
use async_trait::async_trait;
use flume::Sender;
use futures::future::join_all;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) struct OutputBuilder<C, T, U, const N: usize>
where
    C: Write<T, U>,
{
    tx: Vec<Sender<S<T, U>>>,
    client: Arc<Mutex<C>>,
    bootstrap: bool,
}
impl<C, T, U, const N: usize> OutputBuilder<C, T, U, N>
where
    C: Write<T, U>,
{
    pub fn new(client: Arc<Mutex<C>>) -> Self {
        Self {
            tx: Vec::new(),
            client,
            bootstrap: false,
        }
    }
    pub fn senders(self, tx: Vec<Sender<S<T, U>>>) -> Self {
        Self { tx, ..self }
    }
    pub fn bootstrap(self, bootstrap: bool) -> Self {
        Self { bootstrap, ..self }
    }
    pub fn build(self) -> Output<C, T, U, N> {
        Output {
            data: None,
            tx: self.tx,
            client: self.client,
            bootstrap: self.bootstrap,
        }
    }
}

/// [Actor](crate::Actor)s output
pub(crate) struct Output<C, T, U, const N: usize>
where
    C: Write<T, U>,
{
    data: Option<S<T, U>>,
    tx: Vec<Sender<S<T, U>>>,
    client: Arc<Mutex<C>>,
    bootstrap: bool,
}
impl<C, T, U, const N: usize> Output<C, T, U, N>
where
    C: Write<T, U>,
{
    /// Creates a new output from a [Sender] and data [Default]
    pub fn builder(client: Arc<Mutex<C>>) -> OutputBuilder<C, T, U, N> {
        OutputBuilder::new(client)
    }
}
impl<C, T, U, const N: usize> Who<U> for Output<C, T, U, N> where C: Write<T, U> {}

#[async_trait]
pub(crate) trait OutputObject: Send + Sync {
    async fn send(&mut self) -> Result<()>;
    fn bootstrap(&self) -> bool;
    fn len(&self) -> usize;
    fn who(&self) -> String;
}
#[async_trait]
impl<C, T, U, const N: usize> OutputObject for Output<C, T, U, N>
where
    C: Write<T, U> + Send,
    T: Send + Sync,
    U: Send + Sync,
{
    /// Sends output data
    async fn send(&mut self) -> Result<()> {
        self.data = (*self.client.lock().await).write();
        if let Some(data) = &self.data {
            log::debug!("{} sending", Who::who(self));
            let futures: Vec<_> = self
                .tx
                .iter()
                .map(|tx| tx.send_async(data.clone()))
                .collect();
            join_all(futures)
                .await
                .into_iter()
                .collect::<std::result::Result<Vec<()>, flume::SendError<_>>>()
                .map_err(|_| flume::SendError(()))?;
            log::debug!("{} sent", Who::who(self));
            Ok(())
        } else {
            for tx in &self.tx {
                drop(tx);
            }
            Err(ActorError::Disconnected(Who::who(self)))
        }
    }
    /// Bootstraps output
    fn bootstrap(&self) -> bool {
        self.bootstrap
    }
    fn who(&self) -> String {
        Who::who(self)
    }

    fn len(&self) -> usize {
        self.tx.len()
    }
}
