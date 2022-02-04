//! [Actor](crate::Actor)s [Input]/[Output]

use crate::{
    clients::{ClientGeneric, Querializer},
    ActorError, Client, Result,
};
use flume::{Receiver, Sender};
use futures::future::join_all;
use std::{any::Any, marker::PhantomData, ops::Deref, sync::Arc};

pub enum Void {}

/// [Input]/[Output] data
///
/// `N` is the data transfer rate
#[derive(Debug, Default)]
pub struct Data<T, I = Void>(T, PhantomData<I>);
impl<T> Deref for Data<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> From<&Data<Vec<T>>> for Vec<T>
where
    T: Clone,
{
    fn from(data: &Data<Vec<T>>) -> Self {
        data.to_vec()
    }
}
impl<T, I> From<T> for Data<T, I> {
    fn from(u: T) -> Self {
        Data(u, PhantomData)
    }
}
/*
pub trait Wrap<T> {
    fn wrap(data: T) -> Self;
}

impl<T> Wrap<T> for Vec<S<T>> {
    fn wrap(u: T) -> Self {
        vec![Arc::new(Data(u))]
    }
}
*/
pub trait DataObjectToAny: 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> DataObjectToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
pub trait DataObject: Send + Sync {
    fn consumer(&self, client: &mut dyn Client);
}
impl<T, I> DataObject for Data<T, I>
where
    T: 'static + Send + Sync,
    I: 'static + Send + Sync,
{
    fn consumer(&self, client: &mut dyn Client) {
        client.consume(self.clone());
    }
}
impl<T, I> Querializer for Data<T, I> {}

pub type S<T> = Arc<Data<T>>;

/// [Actor](crate::Actor)s input
#[derive(Debug)]
pub struct Input<const N: usize> {
    rx: Receiver<Arc<dyn DataObject>>,
}
impl<const N: usize> Input<N> {
    /// Creates a new intput from a [Receiver] and data [Default]
    pub fn new(rx: Receiver<Arc<dyn DataObject>>) -> Self {
        Self { rx }
    }
    /// Receives output data
    pub async fn recv(&mut self) -> Result<Arc<dyn DataObject>> {
        Ok(self.rx.recv_async().await?)
    }
}

/// [Actor](crate::Actor)s output
#[derive(Debug)]
pub struct Output<const N: usize> {
    tx: Vec<Sender<Arc<dyn DataObject>>>,
}
impl<const N: usize> Output<N> {
    /// Creates a new output from a [Sender] and data [Default]
    pub fn new(tx: Vec<Sender<Arc<dyn DataObject>>>) -> Self {
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
    pub async fn send(&self, data: Option<Arc<dyn DataObject>>) -> Result<Vec<()>> {
        match data {
            Some(data) => {
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
            None => Err(ActorError::NoData),
        }
    }
}
/// Returns one output connected to multiple inputs
pub fn channels<const N: usize>(n_inputs: usize) -> (Output<N>, Vec<Input<N>>) {
    let mut txs = vec![];
    let mut inputs = vec![];
    for _ in 0..n_inputs {
        let (tx, rx) = flume::bounded::<Arc<dyn DataObject>>(1);
        txs.push(tx);
        inputs.push(Input::new(rx));
    }
    (Output::new(txs), inputs)
}
/// Returns a pair of connected input/output
pub fn channel<const N: usize>() -> (Output<N>, Input<N>) {
    let (output, mut inputs) = channels(1);
    (output, inputs.pop().unwrap())
}
