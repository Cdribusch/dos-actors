use crate::{io::*, ActorError, Client, Result};
use futures::future::join_all;
use std::{fmt::Display, marker::PhantomData};

/// Builder for an actor without outputs
pub struct Terminator<I, const NI: usize>(PhantomData<I>);
impl<I, const NI: usize> Terminator<I, NI>
where
    I: Default + std::fmt::Debug,
{
    /// Return an actor without outputs
    pub fn build() -> Actor<I, (), NI, 0> {
        Actor::new()
    }
}

/// Builder for an actor without inputs
pub struct Initiator<O, const NO: usize>(PhantomData<O>);
impl<O, const NO: usize> Initiator<O, NO>
where
    O: Default + std::fmt::Debug,
{
    /// Return an actor without inputs
    pub fn build() -> Actor<(), O, 0, NO> {
        Actor::new()
    }
}

/// Task management abstraction
#[derive(Debug)]
pub struct Actor<I, O, const NI: usize, const NO: usize>
where
    I: Default,
    O: Default + std::fmt::Debug,
{
    pub inputs: Option<Vec<Input<I, NI>>>,
    pub outputs: Option<Vec<Output<O, NO>>>,
    tag: Option<String>,
}

impl<I, O, const NI: usize, const NO: usize> Display for Actor<I, O, NI, NO>
where
    I: Default + std::fmt::Debug,
    O: Default + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.tag.as_ref().unwrap_or(&"Actor".to_string()))?;
        if let Some(inputs) = self.inputs.as_ref() {
            writeln!(f, " - inputs  #{:>1}", inputs.len())?;
        }
        if let Some(outputs) = self.outputs.as_ref() {
            writeln!(
                f,
                " - outputs #{:>1} as {:?}",
                outputs.len(),
                outputs.iter().map(|x| x.len()).collect::<Vec<usize>>()
            )?
        }
        Ok(())
    }
}

impl<I, O, const NI: usize, const NO: usize> Actor<I, O, NI, NO>
where
    I: Default + std::fmt::Debug,
    O: Default + std::fmt::Debug,
{
    /// Creates a new empty [Actor]
    pub fn new() -> Self {
        Self {
            inputs: None,
            outputs: None,
            tag: None,
        }
    }
    pub fn tag<S: Into<String>>(self, tag: S) -> Self {
        Self {
            tag: Some(tag.into()),
            ..self
        }
    }
    // Drops all [Actor::outputs] senders
    fn disconnect(&mut self) -> &mut Self {
        if let Some(outputs) = self.outputs.as_mut() {
            outputs.iter_mut().for_each(|output| output.disconnect())
        }
        self
    }
    /// Gathers all the inputs from other [Actor] outputs
    pub async fn collect(&mut self) -> Result<Vec<S<I>>> {
        let futures: Vec<_> = self
            .inputs
            .as_mut()
            .ok_or(ActorError::NoInputs)?
            .iter_mut()
            .map(|input| input.recv())
            .collect();
        match join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()
        {
            Err(ActorError::DropRecv(e)) => {
                self.disconnect();
                Err(ActorError::DropRecv(e))
            }
            Err(e) => Err(e),
            Ok(data) => Ok(data),
        }
    }
    /// Sends the outputs to other [Actor] inputs
    pub async fn distribute(&mut self, data: Option<Vec<S<O>>>) -> Result<&Self> {
        if let Some(data) = data {
            //self.set_data(data);
            let futures: Vec<_> = self
                .outputs
                .as_ref()
                .ok_or(ActorError::NoOutputs)?
                .iter()
                .zip(data.into_iter())
                .map(|(output, data)| output.send(data))
                .collect();
            join_all(futures)
                .await
                .into_iter()
                .collect::<Result<Vec<_>>>()?;
            Ok(self)
        } else {
            self.disconnect();
            Err(ActorError::Disconnected)
        }
    }
    /// Runs the [Actor] infinite loop
    ///
    /// The loop ends when the client data is [None] or when either the sending of receiving
    /// end of a channel is dropped
    pub async fn run<C: Client<I = I, O = O>>(&mut self, client: &mut C) -> Result<()> {
        match (self.inputs.as_ref(), self.outputs.as_ref()) {
            (Some(_), Some(_)) => {
                if NO >= NI {
                    // Decimation
                    loop {
                        for _ in 0..NO / NI {
                            client.consume(self.collect().await?).update();
                        }
                        self.distribute(client.produce()).await?;
                    }
                } else {
                    // Upsampling
                    loop {
                        client.consume(self.collect().await?).update();
                        for _ in 0..NI / NO {
                            self.distribute(client.produce()).await?;
                        }
                    }
                }
            }
            (None, Some(_)) => loop {
                // Initiator
                self.distribute(client.update().produce()).await?;
            },
            (Some(_), None) => loop {
                // Terminator
                match self.collect().await {
                    Ok(data) => {
                        client.consume(data).update();
                    }
                    Err(e) => break Err(e),
                }
            },
            (None, None) => Ok(()),
        }
    }
}
impl<I, O, const NI: usize, const NO: usize> Actor<I, O, NI, NO>
where
    I: Default + std::fmt::Debug,
    O: Default + std::fmt::Debug,
    Vec<O>: Clone,
{
    /// Bootstraps an actor outputs
    pub async fn bootstrap(&mut self, data: Option<Vec<S<O>>>) -> Result<()> {
        Ok(if NO >= NI {
            self.distribute(data).await?;
        } else {
            for _ in 0..NI / NO {
                self.distribute(data.clone()).await?;
            }
        })
    }
}
