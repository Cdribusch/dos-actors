#[cfg(feature = "windloads")]
pub mod windloads;

/// Client method specifications
pub trait Client: std::fmt::Debug {
    type I;
    type O;
    /// Processes the [Actor] [inputs](Actor::inputs) for the client
    fn consume(&mut self, _data: Vec<&Self::I>) -> &mut Self {
        self
    }
    /// Generates the [outputs](Actor::outputs) from the client
    fn produce(&mut self) -> Option<Vec<Self::O>> {
        Default::default()
    }
    /// Updates the state of the client
    fn update(&mut self) -> &mut Self {
        self
    }
}

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
impl<T> Client for Logging<T>
where
    T: std::fmt::Debug + Clone,
{
    type I = T;
    type O = ();
    fn consume(&mut self, data: Vec<&Self::I>) -> &mut Self {
        self.0.extend(data.into_iter().cloned());
        self
    }
}

/// Sample-and-hold rate transionner
#[derive(Debug, Default)]
pub struct Sampler<T>(Vec<T>);
impl<T> Client for Sampler<T>
where
    T: std::fmt::Debug + Clone,
{
    type I = T;
    type O = T;
    fn consume(&mut self, data: Vec<&Self::I>) -> &mut Self {
        self.0 = data.into_iter().cloned().collect();
        self
    }
    fn produce(&mut self) -> Option<Vec<Self::O>> {
        Some(self.0.drain(..).collect())
    }
}