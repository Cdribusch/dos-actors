//! [Actor](crate::Actor)s [Client]s
//!
//! The module holds the trait [Client] which methods are called
//! by the [Actor](crate::Actor)s client that is passed to the
//! [Actor::run](crate::Actor::run) method
//!
//! A few clients are defined:
//!  - [Logging] that accumulates the data received by a [Terminator](crate::Terminator)
//! into a [Vec]tor
//!  - [Sampler] that moves the data unmodified from inputs to outputs, useful for rate transitions.
//!  - [Signals] that generates some predefined signals

#[cfg(feature = "windloads")]
pub mod windloads;

#[cfg(feature = "fem")]
pub mod fem;

#[cfg(feature = "mount-ctrl")]
pub mod mount;

#[cfg(feature = "m1-ctrl")]
pub mod m1;

#[cfg(feature = "apache-arrow")]
pub mod arrow_client;

//pub mod signals;
//pub use signals::{Signal, Signals};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("cannot open a parquet file")]
    ArrowToFile(#[from] std::io::Error),
    #[cfg(feature = "apache-arrow")]
    #[error("cannot build Arrow data")]
    ArrowError(#[from] arrow::error::ArrowError),
    #[cfg(feature = "apache-arrow")]
    #[error("cannot save data to Parquet")]
    ParquetError(#[from] parquet::errors::ParquetError),
}

use std::sync::Arc;

use crate::io;
/// Client method specifications
pub trait Client {
    /// Processes the [Actor](crate::Actor) [inputs](crate::Actor::inputs) for the client
    fn consume(&mut self, _data: Arc<dyn io::DataObject>) {}
    /// Generates the [outputs](crate::Actor::outputs) from the client
    fn produce(&mut self) -> Option<Arc<dyn io::DataObject>> {
        None
    }
    /// Updates the state of the client
    fn update(&mut self) {}
}

pub trait Consumer<T = (), I = io::Void> {
    /// Processes the [Actor](crate::Actor) [inputs](crate::Actor::inputs) for the client
    fn consume(&mut self, _data: Arc<dyn io::DataObject>) -> &mut Self {
        self
    }
}
impl<T, I> Consumer<T, I> for Box<dyn Client> {}

pub trait Producer<T = (), I = io::Void> {
    /// Generates the [outputs](crate::Actor::outputs) from the client
    fn produce(&mut self) -> Option<Arc<dyn io::DataObject>> {
        None
    }
}
/*
/// Simple data logging
///
/// Accumulates all the inputs in a single [Vec]
#[derive(Default, Debug)]
pub struct Logging<T>(Vec<T>);
impl<T> std::ops::Deref for Logging<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> Client for Logging<Vec<T>>
where
    T: std::fmt::Debug + Clone,
{
    type I = Vec<T>;
    type O = ();
    fn consume(&mut self, data: Vec<io::S<Self::I>>) -> &mut Self {
        log::debug!(
            "receive #{} inputs: {:?}",
            data.len(),
            data.iter().map(|x| x.len()).collect::<Vec<usize>>()
        );
        self.0
            .push(data.into_iter().flat_map(|x| (**x).clone()).collect());
        self
    }
}

/// Sample-and-hold rate transionner
#[derive(Debug, Default)]
pub struct Sampler<T: Default>(Vec<io::S<T>>);
impl<T> Client for Sampler<T>
where
    T: std::fmt::Debug + Clone + Default,
{
    type I = T;
    type O = T;
    fn consume(&mut self, data: Vec<io::S<Self::I>>) -> &mut Self {
        self.0 = data;
        self
    }
    fn produce(&mut self) -> Option<Vec<io::S<Self::O>>> {
        Some(self.0.clone())
    }
}
*/
