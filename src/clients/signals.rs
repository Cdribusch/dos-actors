use std::{ops::Add, sync::Arc};

#[cfg(feature = "noise")]
use rand_distr::{Distribution, Normal, NormalError};

/// Signal types
#[derive(Debug, Clone)]
pub enum Signal {
    /// A constant signal
    Constant(f64),
    /// A sinusoidal signal
    Sinusoid {
        amplitude: f64,
        sampling_frequency_hz: f64,
        frequency_hz: f64,
        phase_s: f64,
    },
    /// White noise
    #[cfg(feature = "noise")]
    WhiteNoise(Normal<f64>),
    /// A simphony?
    Composite(Vec<Signal>),
}
#[cfg(feature = "noise")]
impl Signal {
    /// Create a white noise signal with a standard deviation equal to one
    pub fn white_noise() -> Result<Self, NormalError> {
        Ok(Signal::WhiteNoise(Normal::new(0f64, 1f64)?))
    }
    /// Sets white noise standard deviation
    pub fn std_dev(self, sigma: f64) -> Result<Self, NormalError> {
        if let Signal::WhiteNoise(noise) = self {
            Ok(Signal::WhiteNoise(Normal::new(noise.mean(), sigma)?))
        } else {
            Ok(self)
        }
    }
    /// Adds bias to white noise
    pub fn bias(self, bias: f64) -> Result<Self, NormalError> {
        if let Signal::WhiteNoise(noise) = self {
            Ok(Signal::WhiteNoise(Normal::new(bias, noise.std_dev())?))
        } else {
            Ok(self)
        }
    }
}
impl Signal {
    /// Returns the signal value at step `i`
    pub fn get(&self, i: usize) -> f64 {
        use Signal::*;
        match self {
            Constant(val) => *val,
            Sinusoid {
                amplitude,
                sampling_frequency_hz,
                frequency_hz,
                phase_s,
            } => {
                (2f64
                    * std::f64::consts::PI
                    * (phase_s + i as f64 * frequency_hz / sampling_frequency_hz))
                    .sin()
                    * amplitude
            }
            #[cfg(feature = "noise")]
            WhiteNoise(noise) => noise.sample(&mut rand::thread_rng()),
            Composite(signals) => signals.iter().map(|signal| signal.get(i)).sum(),
        }
    }
}

/// Signals generator
#[derive(Debug, Default)]
pub struct Signals {
    outputs_size: Vec<usize>,
    signals: Vec<Vec<Signal>>,
    pub step: usize,
    pub n_step: usize,
}
impl Signals {
    /// Create new signals
    pub fn new(outputs_size: Vec<usize>, n_step: usize) -> Self {
        let signal: Vec<_> = outputs_size
            .iter()
            .map(|&n| vec![Signal::Constant(0f64); n])
            .collect();
        Self {
            outputs_size,
            signals: signal,
            step: 0,
            n_step,
        }
    }
    /// Sets the type of signals
    pub fn signals(self, signal: Signal) -> Self {
        let signal: Vec<_> = self
            .outputs_size
            .iter()
            .map(|&n| vec![signal.clone(); n])
            .collect();
        Self {
            signals: signal,
            ..self
        }
    }
    /// Sets the type of signals of one output
    pub fn output_signals(self, output: usize, output_signals: Signal) -> Self {
        let mut signals = self.signals;
        signals[output] = vec![output_signals; self.outputs_size[output]];
        Self { signals, ..self }
    }
    /// Sets the type of signals of one output index
    pub fn output_signal(self, output: usize, output_i: usize, signal: Signal) -> Self {
        let mut signals = self.signals;
        signals[output][output_i] = signal;
        Self { signals, ..self }
    }
}

impl Add for Signal {
    type Output = Signal;

    fn add(self, rhs: Self) -> Self::Output {
        if let Signal::Composite(mut signals) = self {
            signals.push(rhs);
            Signal::Composite(signals)
        } else {
            Signal::Composite(vec![self, rhs])
        }
    }
}

use crate::io;

impl super::Client for Signals {
    type I = ();
    type O = Vec<f64>;
    fn produce(&mut self) -> Option<Vec<io::S<Self::O>>> {
        log::debug!("produce {:?}", self.outputs_size);
        if self.step < self.n_step {
            let i = self.step;
            self.step += 1;
            Some(
                self.signals
                    .iter()
                    .map(|signals| {
                        let data: io::Data<Self::O> = signals
                            .iter()
                            .map(|signal| signal.get(i))
                            .collect::<Vec<_>>()
                            .into();
                        Arc::new(data)
                    })
                    .collect(),
            )
        } else {
            None
        }
    }
}
