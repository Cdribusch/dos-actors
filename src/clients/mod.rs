/*!
# [Actor](crate::Actor)s clients

The module holds the implementation of the different clients that can be assigned to [Actor]s.

Any structure can become a client to an Actor if it implements the [Update] trait with either or both [Read] and [Write] traits.

# Example

## Logging

A simple logger with a single entry:
```
use dos_actors::prelude::*;
let logging = Logging::<f64>::default();
```
A logger with 2 entries and pre-allocated with 1000 elements:
```
use dos_actors::prelude::*;
let logging = Logging::<f64>::default().n_entry(2).capacity(1_000);
```
## Signals

A constant signal for 100 steps
```
use dos_actors::prelude::*;
let signal = Signals::new(1, 100).signals(Signal::Constant(3.14));
```

A 2 outputs signal made of a constant and a sinusoid for 100 steps
```
use dos_actors::prelude::*;
let signal = Signals::new(2, 100)
               .output_signal(0, Signal::Constant(3.14))
               .output_signal(1, Signal::Sinusoid{
                                        amplitude: 1f64,
                                        sampling_frequency_hz: 1000f64,
                                        frequency_hz: 20f64,
                                        phase_s: 0f64
               });
```
## Rate transitionner

A sample-and-hold rate transition for a named output/input pair sampling a [Vec]
```
use dos_actors::prelude::*;
enum MyIO {};
let sampler = Sampler::<Vec<f64>, MyIO>::default();
```

[Actor]: crate::actor
*/

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

#[cfg(feature = "fsm")]
pub mod fsm;

#[cfg(feature = "crseo")]
pub mod ceo;

#[cfg(feature = "lom")]
pub mod lom;

use crate::{
    io::{Data, Read, Write},
    Update,
};
use std::{
    any::type_name,
    fmt::Display,
    marker::PhantomData,
    mem::take,
    ops::{Add, Mul, Sub, SubAssign},
    sync::Arc,
};
mod signals;
#[doc(inline)]
pub use signals::{Signal, Signals};

/// Simple data logging
///
/// Accumulates all the inputs in a single [Vec]
#[derive(Debug)]
pub struct Logging<T> {
    data: Vec<T>,
    n_sample: usize,
    n_entry: usize,
}

impl<T> std::ops::Deref for Logging<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> Default for Logging<T> {
    fn default() -> Self {
        Self {
            n_entry: 1,
            data: Vec::new(),
            n_sample: 0,
        }
    }
}
impl<T> Logging<T> {
    /// Sets the # of entries to be logged (default: 1)
    pub fn n_entry(self, n_entry: usize) -> Self {
        Self { n_entry, ..self }
    }
    /// Pre-allocates the size of the vector holding the data
    pub fn capacity(self, capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            ..self
        }
    }
}

impl<T> Logging<T> {
    /// Returns the # of time samples
    pub fn len(&self) -> usize {
        self.n_sample / self.n_entry
    }
    /// Returns the sum of the entry sizes
    pub fn n_data(&self) -> usize {
        self.data.len() / self.len()
    }
    /// Checks if the logger is empty
    pub fn is_empty(&self) -> bool {
        self.n_sample == 0
    }
    /// Returns data chunks the size of the entries
    pub fn chunks(&self) -> impl Iterator<Item = &[T]> {
        self.data.chunks(self.n_data())
    }
}

impl<T> Display for Logging<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Logging: ({}x{})={}",
            self.n_data(),
            self.len(),
            self.data.len()
        )
    }
}

impl<T> Update for Logging<T> {}
impl<T: Clone, U> Read<Vec<T>, U> for Logging<T> {
    fn read(&mut self, data: Arc<Data<Vec<T>, U>>) {
        log::debug!("receive {} input: {:}", type_name::<U>(), data.len(),);
        self.data.extend((**data).clone());
        self.n_sample += 1;
    }
}

/// Sample-and-hold rate transitionner
#[derive(Debug)]
pub struct Sampler<T, U, V = U> {
    input: Arc<Data<T, U>>,
    output: PhantomData<V>,
}
impl<T: Default, U, V> Default for Sampler<T, U, V> {
    fn default() -> Self {
        Self {
            input: Arc::new(Data::new(T::default())),
            output: PhantomData,
        }
    }
}
impl<T, U, V> Update for Sampler<T, U, V> {}
impl<T, U, V> Read<T, U> for Sampler<T, U, V> {
    fn read(&mut self, data: Arc<Data<T, U>>) {
        self.input = data;
    }
}
impl<T: Clone, U, V> Write<T, V> for Sampler<T, U, V> {
    fn write(&mut self) -> Option<Arc<Data<T, V>>> {
        Some(Arc::new(Data::new((**self.input).clone())))
    }
}

/// Concatenates data into a [Vec]
pub struct Concat<T>(Vec<T>);
impl<T: Default> Default for Concat<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
impl<T> Update for Concat<T> {}
impl<T: Clone + Default, U> Read<T, U> for Concat<T> {
    fn read(&mut self, data: Arc<Data<T, U>>) {
        self.0.push((*data).clone());
    }
}
impl<T: Clone, U> Write<Vec<T>, U> for Concat<T> {
    fn write(&mut self) -> Option<Arc<Data<Vec<T>, U>>> {
        Some(Arc::new(Data::new(take(&mut self.0))))
    }
}

/// Integral controller
#[derive(Default)]
pub struct Integrator<T, U> {
    gain: Vec<T>,
    mem: Vec<T>,
    zero: Vec<T>,
    uid: PhantomData<U>,
}
impl<T, U> Integrator<T, U>
where
    T: Default + Clone,
{
    /// Creates a new integral controller
    pub fn new(n_data: usize) -> Self {
        Self {
            gain: vec![Default::default(); n_data],
            mem: vec![Default::default(); n_data],
            zero: vec![Default::default(); n_data],
            uid: PhantomData,
        }
    }
    /// Sets a unique gain
    pub fn gain(self, gain: T) -> Self {
        Self {
            gain: vec![gain; self.mem.len()],
            ..self
        }
    }
    /// Sets the gain vector
    pub fn gain_vector(self, gain: Vec<T>) -> Self {
        assert_eq!(
            gain.len(),
            self.mem.len(),
            "gain vector length error: expected {} found {}",
            gain.len(),
            self.mem.len()
        );
        Self { gain, ..self }
    }
    /// Sets the integrator zero point
    pub fn zero(self, zero: Vec<T>) -> Self {
        Self { zero, ..self }
    }
}
impl<T, U> Update for Integrator<T, U> {}
impl<T, U> Read<Vec<T>, U> for Integrator<T, U>
where
    T: Copy + Mul<Output = T> + Sub<Output = T> + SubAssign,
{
    fn read(&mut self, data: Arc<Data<Vec<T>, U>>) {
        self.mem
            .iter_mut()
            .zip(&self.gain)
            .zip(&self.zero)
            .zip(&**data)
            .for_each(|(((x, g), z), u)| *x -= *g * (*u - *z));
    }
}
impl<T, V, U> Write<Vec<T>, V> for Integrator<T, U>
where
    T: Copy + Add<Output = T>,
{
    fn write(&mut self) -> Option<Arc<Data<Vec<T>, V>>> {
        let y: Vec<T> = self
            .mem
            .iter()
            .zip(&self.zero)
            .map(|(m, z)| *m + *z)
            .collect();
        Some(Arc::new(Data::new(y)))
    }
}
