use crate::{io::*, ActorError, Client, Result};
use futures::future::join_all;
use std::fmt::Display;

/// Builder for an actor without outputs
pub struct Terminator<const NI: usize>();
impl<const NI: usize> Terminator<NI> {
    /// Return an actor without outputs
    pub fn build() -> Actor<NI, 0> {
        Actor::new()
    }
}

/// Builder for an actor without inputs
pub struct Initiator<const NO: usize>();
impl<const NO: usize> Initiator<NO> {
    /// Return an actor without inputs
    pub fn build() -> Actor<0, NO> {
        Actor::new()
    }
}

/// Task management abstraction
#[derive(Debug)]
pub struct Actor<const NI: usize, const NO: usize> {
    pub inputs: Option<Vec<Input<NI>>>,
    pub outputs: Option<Vec<Output<NO>>>,
    tag: Option<String>,
}

impl<const NI: usize, const NO: usize> Display for Actor<NI, NO> {
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

impl<const NI: usize, const NO: usize> Actor<NI, NO> {
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
    pub async fn collect<C: Client>(&mut self, client: &mut C) -> Result<&mut Self> {
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
            Ok(data) => {
                for data in data.into_iter() {
                    client.consume(data);
                }
                Ok(self)
            }
        }
    }
    /// Sends the outputs to other [Actor] inputs
    pub async fn distribute<C: Client>(&mut self, client: &mut C) -> Result<&Self> {
        let futures: Vec<_> = self
            .outputs
            .as_ref()
            .ok_or(ActorError::NoOutputs)?
            .iter()
            .map(|output| output.send(client.produce()))
            .collect();
        match join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()
        {
            Ok(_) => Ok(self),
            Err(_) => {
                self.disconnect();
                Err(ActorError::Disconnected)
            }
        }
    }
    /// Runs the [Actor] infinite loop
    ///
    /// The loop ends when the client data is [None] or when either the sending of receiving
    /// end of a channel is dropped
    pub async fn run<C: Client>(&mut self, client: &mut C) -> Result<()> {
        match (self.inputs.as_ref(), self.outputs.as_ref()) {
            (Some(_), Some(_)) => {
                if NO >= NI {
                    // Decimation
                    loop {
                        for _ in 0..NO / NI {
                            self.collect(client).await?;
                            client.update();
                        }
                        self.distribute(client).await?;
                    }
                } else {
                    // Upsampling
                    loop {
                        self.collect(client).await?;
                        client.update();
                        for _ in 0..NI / NO {
                            self.distribute(client).await?;
                        }
                    }
                }
            }
            (None, Some(_)) => loop {
                // Initiator
                client.update();
                self.distribute(client).await?;
            },
            (Some(_), None) => loop {
                // Terminator
                match self.collect(client).await {
                    Ok(_) => (),
                    Err(e) => break Err(e),
                }
            },
            (None, None) => Ok(()),
        }
    }
}
impl<const NI: usize, const NO: usize> Actor<NI, NO> {
    /// Bootstraps an actor outputs
    pub async fn bootstrap<C: Client>(&mut self, client: &mut C) -> Result<()> {
        Ok(if NO >= NI {
            self.distribute(client).await?;
        } else {
            for _ in 0..NI / NO {
                self.distribute(client).await?;
            }
        })
    }
}
