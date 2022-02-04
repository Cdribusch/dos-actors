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
pub trait Querializer {}

pub trait ClientGeneric {
    // Not object safe because of this generic method.
    fn consume<Q: Querializer>(&self, data: Q);
}

impl<'a, T: ?Sized> Querializer for &'a T where T: Querializer {}
//impl<'a, T: ?Sized> DataObject for &'a T where T: DataObject {}
//impl<Q: Querializer, I: Identifier> DataObject for Data<Q, I> {}

impl<'a, T: ?Sized> ClientGeneric for Box<T>
where
    T: ClientGeneric,
{
    fn consume<Q: Querializer>(&self, data: Q) {
        (**self).consume(data)
    }
}

/////////////////////////////////////////////////////////////////////
// This is an object-safe equivalent that interoperates seamlessly.
pub trait Client {
    fn erased_fn(&self, data: &dyn Querializer);
    fn produce(&mut self) -> Option<Arc<dyn io::DataObject>> {
        None
    }
    /// Updates the state of the client
    fn update(&mut self) {}
}

impl ClientGeneric for dyn Client + '_ {
    // Depending on the trait method signatures and the upstream
    // impls, could also implement for:
    //
    //   - &'a dyn Client
    //   - &'a (dyn Client + Send)
    //   - &'a (dyn Client + Sync)
    //   - &'a (dyn Client + Send + Sync)
    //   - Box<dyn Client>
    //   - Box<dyn Client + Send>
    //   - Box<dyn Client + Sync>
    //   - Box<dyn Client + Send + Sync>
    fn consume<Q: Querializer>(&self, data: Q) {
        self.erased_fn(&data)
    }
}

impl<T> Client for T
where
    T: ClientGeneric,
{
    fn erased_fn(&self, data: &dyn Querializer) {
        self.consume(data)
    }
}
/*
pub trait Client {
    /// Processes the [Actor](crate::Actor) [inputs](crate::Actor::inputs) for the client
    //fn consume<Q>(&mut self, _data: Q) {}
    fn consume(&mut self, _data: Arc<dyn io::DataObject>) {}
    /// Generates the [outputs](crate::Actor::outputs) from the client
    fn produce(&mut self) -> Option<Arc<dyn io::DataObject>> {
        None
    }
    /// Updates the state of the client
    fn update(&mut self) {}
}
*/

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
