//! [Actor](crate::Actor)s [Input]/[Output]

use crate::Result;
use flume::{Receiver, Sender};
use futures::future::join_all;
use std::{ops::Deref, sync::Arc};

/// [Input]/[Output] data
///
/// `N` is the data transfer rate
#[derive(Debug, Default)]
pub struct Data<T: Default>(pub T);
impl<T> Deref for Data<T>
where
    T: Default,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> From<&Data<Vec<T>>> for Vec<T>
where
    T: Default + Clone,
{
    fn from(data: &Data<Vec<T>>) -> Self {
        data.to_vec()
    }
}
impl<T> From<T> for Data<T>
where
    T: Default,
{
    fn from(u: T) -> Self {
        Data(u)
    }
}
pub trait Wrap<T> {
    fn wrap(data: T) -> Self;
}

impl<T> Wrap<T> for Vec<S<T>>
where
    T: Default,
{
    fn wrap(u: T) -> Self {
        vec![Arc::new(Data(u))]
    }
}

pub type S<T> = Arc<Data<T>>;

/// [Actor](crate::Actor)s input
#[derive(Debug)]
pub struct Input<T: Default, const N: usize> {
    rx: Receiver<S<T>>,
}
impl<T, const N: usize> Input<T, N>
where
    T: Default,
{
    /// Creates a new intput from a [Receiver] and data [Default]
    pub fn new(rx: Receiver<S<T>>) -> Self {
        Self { rx }
    }
    /// Receives output data
    pub async fn recv(&mut self) -> Result<S<T>> {
        Ok(self.rx.recv_async().await?)
    }
}

/// [Actor](crate::Actor)s output
#[derive(Debug)]
pub struct Output<T: Default, const N: usize> {
    tx: Vec<Sender<S<T>>>,
}
impl<T: Default, const N: usize> Output<T, N> {
    /// Creates a new output from a [Sender] and data [Default]
    pub fn new(tx: Vec<Sender<S<T>>>) -> Self {
        Self { tx }
    }
    pub fn len(&self) -> usize {
        self.tx.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Drops all senders
    pub fn disconnect(&mut self) {
        self.tx.iter_mut().for_each(drop);
    }
    /// Sends output data
    pub async fn send(&self, data: S<T>) -> Result<Vec<()>> {
        let futures: Vec<_> = self
            .tx
            .iter()
            .map(|tx| tx.send_async(data.clone()))
            .collect();
        Ok(join_all(futures)
            .await
            .into_iter()
            .collect::<std::result::Result<Vec<()>, flume::SendError<_>>>()
            .map_err(|_| flume::SendError(()))?)
    }
}
/// Returns one output connected to multiple inputs
pub fn channels<T, const N: usize>(n_inputs: usize) -> (Output<T, N>, Vec<Input<T, N>>)
where
    T: Default,
{
    let mut txs = vec![];
    let mut inputs = vec![];
    for _ in 0..n_inputs {
        let (tx, rx) = flume::bounded::<S<T>>(1);
        txs.push(tx);
        inputs.push(Input::new(rx));
    }
    (Output::new(txs), inputs)
}
/// Returns a pair of connected input/output
pub fn channel<T, const N: usize>() -> (Output<T, N>, Input<T, N>)
where
    T: Default,
{
    let (output, mut inputs) = channels(1);
    (output, inputs.pop().unwrap())
}
