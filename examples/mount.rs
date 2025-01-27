use std::time::Instant;

use dos_actors::clients::mount::{Mount, MountEncoders, MountTorques};
use dos_actors::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //simple_logger::SimpleLogger::new().env().init().unwrap();
    let sim_sampling_frequency = 1000f64;

    let signals = Signals::new(vec![14], 1001).signals(Signal::Sinusoid {
        amplitude: 1e-6,
        sampling_frequency_hz: sim_sampling_frequency,
        frequency_hz: 10.,
        phase_s: 0.,
    });
    let mut source: Initiator<_> = signals.into();

    let mut mount: Actor<_> = Mount::new().into();

    let logging = Logging::<f64>::default().into_arcx();
    let mut sink = Terminator::new(logging.clone());

    source
        .add_output::<Vec<f64>, MountEncoders>(None)
        .into_input(&mut mount);
    mount
        .add_output::<Vec<f64>, MountTorques>(None)
        .into_input(&mut sink);

    println!("{mount}");

    spawn!(source, mount);
    let now = Instant::now();
    run!(sink);
    println!("Model run in {}ms", now.elapsed().as_millis());

    println!("logging: {}", (*logging.lock().await).len());

    let _: complot::Plot = (
        (*logging.lock().await)
            .chunks(20)
            .enumerate()
            .map(|(i, x)| (i as f64 * sim_sampling_frequency.recip(), x.to_vec())),
        complot::complot!("examples/mount.png", xlabel = "Time [s]", ylabel = ""),
    )
        .into();

    Ok(())
}
