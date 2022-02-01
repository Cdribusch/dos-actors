use dos_actors::prelude::*;
use rand_distr::{Distribution, Normal};
use std::{any::Any, ops::Deref, sync::Arc, time::Instant};

#[derive(Default, Debug)]
struct Signal {
    pub sampling_frequency: f64,
    pub period: f64,
    pub n_step: usize,
    pub step: usize,
}
impl Client for Signal {
    fn produce(&mut self) -> Option<Arc<dyn io::DataObject>> {
        if self.step < self.n_step {
            let value = (2.
                * std::f64::consts::PI
                * self.step as f64
                * (self.sampling_frequency * self.period).recip())
            .sin()
                - 0.25
                    * (2.
                        * std::f64::consts::PI
                        * ((self.step as f64
                            * (self.sampling_frequency * self.period * 0.25).recip())
                            + 0.1))
                        .sin();
            self.step += 1;
            let data: io::Data<f64> = io::Data::from(value);
            Some(Arc::new(data))
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct Filter {
    data: f64,
    noise: Normal<f64>,
    step: usize,
}
impl Default for Filter {
    fn default() -> Self {
        Self {
            data: 0f64,
            noise: Normal::new(0.3, 0.05).unwrap(),
            step: 0,
        }
    }
}
impl Client for Filter {
    fn consume(&mut self, data: Arc<dyn io::DataObject>) {
        let it: &dyn Any = data.as_any();
        match it.downcast_ref::<io::Data<f64>>() {
            Some(data) => {
                self.data = **data;
            }
            None => panic!(),
        }
    }
    fn update(&mut self) {
        self.data += 0.05
            * (2. * std::f64::consts::PI * self.step as f64 * (1e3f64 * 2e-2).recip()).sin()
            + self.noise.sample(&mut rand::thread_rng());
        self.step += 1;
    }
    fn produce(&mut self) -> Option<Arc<dyn io::DataObject>> {
        let data: io::Data<f64> = io::Data::from(self.data);
        Some(Arc::new(data))
    }
}

#[derive(Default, Debug)]
pub struct Logging(Vec<f64>);
impl std::ops::Deref for Logging {
    type Target = Vec<f64>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Client for Logging {
    fn consume(&mut self, data: Arc<dyn io::DataObject>) {
        let it: &dyn Any = data.as_any();
        match it.downcast_ref::<io::Data<f64>>() {
            Some(data) => {
                self.0.push(**data);
            }
            None => panic!(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let n_sample = 2001;
    let sim_sampling_frequency = 1000f64;

    let mut signal = Signal {
        sampling_frequency: sim_sampling_frequency,
        period: 1f64,
        n_step: n_sample,
        step: 0,
    };
    let mut logging = Logging::default();

    /*
    model!{
       - data type: f64
       - actors: source + filter >> sink
       - channels:
          - source => filter => sink
          - source => sink
       - clients:
          - spawn:
            - source, signal
            - filter, Filter::default()
          - run:
            - sink, logging
     }
     */

    let (mut source, mut filter, mut sink) = stage!(source >> filter << sink);

    channel![source => (filter , sink)];
    channel![filter => sink];

    spawn!((source, signal,), (filter, Filter::default(),));
    let now = Instant::now();
    run!(sink, logging);
    println!("Model run in {}ms", now.elapsed().as_millis());

    let _: complot::Plot = (
        logging
            .deref()
            .chunks(2)
            .enumerate()
            .map(|(i, x)| (i as f64 * sim_sampling_frequency.recip(), x.to_vec())),
        None,
    )
        .into();

    Ok(())
}
