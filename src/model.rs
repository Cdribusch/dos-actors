/*!
# Integrated model

The module implements the high-level integrated model interface.
The model is build from a collection of [actor]s.

The model has 4 states:
 1. [Unknown]: model state at its creation
 2. [Ready]: model state after succesfully performing runtime checks on inputs and outputs on all the actors, the model can move to the [Ready] state only from the [Unknown] state
 3. [Running]: model state while all the actors are performing their respective tasks, the model can move to the [Running] state only from the [Ready] state
 4. [Completed]: model state after the succesful completion of the tasks of all the actors, the model can move to the [Completed] state only from the [Running] state

# Example

A 3 actors model with [Signals], [Sampler] and [Logging] clients is build with:
```
use dos_actors::prelude::*;
let mut source: Initiator<_> = Signals::new(1, 100).into();
enum Source {};
let mut sampler: Actor<_, 1, 10> = Sampler::<Vec<f64>, Source>::default().into();
let logging = Logging::<f64>::default().into_arcx();
let mut sink = Terminator::<_, 10>::new(logging);
```
`sampler` decimates `source` with a 1:10 ratio.
The `source` connects to the `sampler` using the empty enum type `Source` as the data identifier.
The source data is then logged into the client of the `sink` actor.
```
# use dos_actors::prelude::*;
# let mut source: Initiator<_> = Signals::new(1, 100).into();
# enum Source {};
# let mut sampler: Actor<_> = Sampler::<Vec<f64>, Source>::default().into();
# let logging = Logging::<f64>::default().into_arcx();
# let mut sink = Terminator::<_>::new(logging);
source.add_output().build::<Vec<f64>, Source>().into_input(&mut sampler);
sampler.add_output().build::<Vec<f64>,Source>().into_input(&mut sink);
```
A [model](crate::model) is build from the set of actors:
```
# use dos_actors::prelude::*;
# let mut source: Initiator<_> = Signals::new(1, 100).into();
# enum Source {};
# let mut sampler: Actor<_> = Sampler::<Vec<f64>, Source>::default().into();
# let logging = Logging::<f64>::default().into_arcx();
# let mut sink = Terminator::<_>::new(logging.clone());
# source.add_output().build::<Vec<f64>, Source>().into_input(&mut sampler);
# sampler.add_output().build::<Vec<f64>,Source>().into_input(&mut sink);
Model::new(vec![Box::new(source), Box::new(sampler), Box::new(sink)]);
```
Actors are checked for inputs/outputs consistencies:
```
# use dos_actors::prelude::*;
# let mut source: Initiator<_> = Signals::new(1, 100).into();
# enum Source {};
# let mut sampler: Actor<_> = Sampler::<Vec<f64>, Source>::default().into();
# let logging = Logging::<f64>::default().into_arcx();
# let mut sink = Terminator::<_>::new(logging.clone());
# source.add_output().build::<Vec<f64>, Source>().into_input(&mut sampler);
# sampler.add_output().build::<Vec<f64>,Source>().into_input(&mut sink);
Model::new(vec![Box::new(source), Box::new(sampler), Box::new(sink)])
       .check()?;
# Ok::<(), dos_actors::model::ModelError>(())
```
The model runs the actor tasks:
```
# tokio_test::block_on(async {
# use dos_actors::prelude::*;
# let mut source: Initiator<_> = Signals::new(1, 100).into();
# enum Source {};
# let mut sampler: Actor<_> = Sampler::<Vec<f64>, Source>::default().into();
# let logging = Logging::<f64>::default().into_arcx();
# let mut sink = Terminator::<_>::new(logging.clone());
# source.add_output().build::<Vec<f64>, Source>().into_input(&mut sampler);
# sampler.add_output().build::<Vec<f64>,Source>().into_input(&mut sink);
Model::new(vec![Box::new(source), Box::new(sampler), Box::new(sink)])
       .check()?
       .run();
# Ok::<(), dos_actors::model::ModelError>(())
# });
```
and wait for the tasks to finish:
```
# tokio_test::block_on(async {
# use dos_actors::prelude::*;
# let mut source: Initiator<_> = Signals::new(1, 100).into();
# enum Source {};
# let mut sampler: Actor<_> = Sampler::<Vec<f64>, Source>::default().into();
# let logging = Logging::<f64>::default().into_arcx();
# let mut sink = Terminator::<_>::new(logging.clone());
# source.add_output().build::<Vec<f64>, Source>().into_input(&mut sampler);
# sampler.add_output().build::<Vec<f64>,Source>().into_input(&mut sink);
Model::new(vec![Box::new(source), Box::new(sampler), Box::new(sink)])
       .check()?
       .run()
       .wait()
       .await?;
# Ok::<(), dos_actors::model::ModelError>(())
# });
```
Once the model run to completion, the data from `logging` is read with:
```
# tokio_test::block_on(async {
# use dos_actors::prelude::*;
# let mut source: Initiator<_> = Signals::new(1, 100).into();
# enum Source {};
# let mut sampler: Actor<_> = Sampler::<Vec<f64>, Source>::default().into();
# let logging = Logging::<f64>::default().into_arcx();
# let mut sink = Terminator::<_>::new(logging.clone());
# source.add_output().build::<Vec<f64>, Source>().into_input(&mut sampler);
# sampler.add_output().build::<Vec<f64>,Source>().into_input(&mut sink);
# Model::new(vec![Box::new(source), Box::new(sampler), Box::new(sink)])
#       .check()?
#       .run()
#       .wait()
#       .await?;
let data: &[f64]  = &logging.lock().await;
# Ok::<(), dos_actors::model::ModelError>(())
# });
```

[actor]: crate::actor
[client]: crate::clients
[Mutex]: tokio::sync::Mutex
[Arc]: std::sync::Arc
[Arcmutex]: crate::ArcMutex
[into_arcx]: crate::ArcMutex::into_arcx
[Signals]: crate::clients::Signals
[Sampler]: crate::clients::Sampler
[Logging]: crate::clients::Logging
*/

use crate::{
    actor::{PlainActor, PlainOutput},
    Task,
};
use chrono::{DateTime, Local, SecondsFormat};
use std::{
    collections::BTreeMap, fs::File, io::Write, marker::PhantomData, path::Path, process::Command,
    time::Instant,
};

#[derive(thiserror::Error, Debug)]
pub enum ModelError {
    #[error("no actors found in the model")]
    NoActors,
    #[error("failed to join the task")]
    TaskError(#[from] tokio::task::JoinError),
    #[error("Actor IO inconsistency")]
    ActorIO(#[from] crate::ActorError),
}

type Result<T> = std::result::Result<T, ModelError>;

/// [Model] initial state
pub enum Unknown {}
/// Valid [Model] state
pub enum Ready {}
/// [Model]ing in-progress state
pub enum Running {}
/// [Model] final state
pub enum Completed {}

type Actors = Vec<Box<dyn Task>>;

/// Actor model
pub struct Model<State> {
    name: Option<String>,
    actors: Option<Actors>,
    task_handles: Option<Vec<tokio::task::JoinHandle<()>>>,
    state: PhantomData<State>,
    start: Instant,
}

#[doc(hidden)]
pub trait UnknownOrReady {}
impl UnknownOrReady for Unknown {}
impl UnknownOrReady for Ready {}
impl<State> Model<State>
where
    State: UnknownOrReady,
{
    /// Returns a [Graph] of the model
    pub fn graph(&self) -> Option<Graph> {
        self.actors
            .as_ref()
            .map(|actors| Graph::new(actors.iter().map(|a| a.as_plain()).collect()))
    }
    /// Produces the model flowchart
    pub fn flowchart(self) -> Self {
        let name = self
            .name
            .clone()
            .unwrap_or_else(|| "integrated_model".to_string());
        let path = Path::new(&name);
        if let Some(graph) = self.graph() {
            graph
                .to_dot(path.with_extension("dot"))
                .expect("Failed to write Graphviz dot file");
            Command::new("neato")
                .arg("-Gstart=rand")
                .arg("-Tsvg")
                .arg("-O")
                .arg(path.with_extension("dot").to_str().unwrap())
                .output()
                .expect("Failed to convert Graphviz dot file to SVG image");
        }
        self
    }
}

impl Model<Unknown> {
    /// Returns a new model
    pub fn new(actors: Actors) -> Self {
        Self {
            name: None,
            actors: Some(actors),
            task_handles: None,
            state: PhantomData,
            start: Instant::now(),
        }
    }
    /// Sets the model name
    pub fn name<S: Into<String>>(self, name: S) -> Self {
        Self {
            name: Some(name.into()),
            ..self
        }
    }
    /// Validates actors inputs and outputs
    pub fn check(self) -> Result<Model<Ready>> {
        match self.actors {
            Some(ref actors) => {
                for actor in actors {
                    actor.check_inputs()?;
                    actor.check_outputs()?;
                }
                Ok(Model::<Ready> {
                    name: self.name,
                    actors: self.actors,
                    task_handles: None,
                    state: PhantomData,
                    start: Instant::now(),
                })
            }
            None => Err(ModelError::NoActors),
        }
    }
}

impl Model<Ready> {
    /// Spawns each actor task
    pub fn run(mut self) -> Model<Running> {
        let mut actors = self.actors.take().unwrap();
        let mut task_handles = vec![];
        while let Some(mut actor) = actors.pop() {
            task_handles.push(tokio::spawn(async move {
                actor.task().await;
            }));
        }
        let now: DateTime<Local> = Local::now();
        println!(
            "[{}<{}>] LAUNCHED",
            self.name
                .as_ref()
                .unwrap_or(&String::from("Model"))
                .to_uppercase(),
            now.to_rfc3339_opts(SecondsFormat::Secs, true),
        );
        Model::<Running> {
            name: self.name,
            actors: None,
            task_handles: Some(task_handles),
            state: PhantomData,
            start: Instant::now(),
        }
    }
}

impl Model<Running> {
    /// Waits for the task of each actor to finish
    pub async fn wait(mut self) -> Result<Model<Completed>> {
        let task_handles = self.task_handles.take().unwrap();
        for task_handle in task_handles.into_iter() {
            task_handle.await?;
        }
        let elapsed_time = Instant::now().duration_since(self.start);
        let now: DateTime<Local> = Local::now();
        println!(
            "[{}<{}>] COMPLETED in {}",
            self.name
                .as_ref()
                .unwrap_or(&String::from("Model"))
                .to_uppercase(),
            now.to_rfc3339_opts(SecondsFormat::Secs, true),
            humantime::format_duration(elapsed_time)
        );
        Ok(Model::<Completed> {
            name: self.name,
            actors: None,
            task_handles: None,
            state: PhantomData,
            start: Instant::now(),
        })
    }
}

/// [Model] network mapping
///
/// The structure is used to build a [Graphviz](https://www.graphviz.org/) diagram of a [Model].
/// A new [Graph] is created with [Model::graph()].
///
/// The model flow chart is written to a SVG image with `neato -Gstart=rand -Tsvg filename.dot > filename.svg`
#[derive(Debug)]
pub struct Graph {
    actors: Vec<PlainActor>,
}
impl Graph {
    fn new(actors: Vec<PlainActor>) -> Self {
        let mut actors = actors;
        actors.iter_mut().for_each(|actor| {
            actor.client = actor
                .client
                .replace("::Controller", "")
                .split('<')
                .next()
                .unwrap()
                .split("::")
                .last()
                .unwrap()
                .to_string();
        });
        Self { actors }
    }
    /// Returns the diagram in the [Graphviz](https://www.graphviz.org/) dot language
    pub fn to_string(&self) -> String {
        use PlainOutput::*;
        let mut lookup: BTreeMap<usize, usize> = BTreeMap::new();
        let mut colors = (1usize..=8).cycle();
        let outputs: Vec<_> = self
            .actors
            .iter()
            .filter_map(|actor| {
                actor.outputs.as_ref().map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| {
                            let color = lookup
                                .entry(actor.outputs_rate)
                                .or_insert_with(|| colors.next().unwrap());
                            match output {
                                Bootstrap(output) => format!(
                                    "{0} -> {1} [color={2}, style=bold];",
                                    actor.client,
                                    output.split("::").last().unwrap(),
                                    color
                                ),
                                Regular(output) => format!(
                                    "{0} -> {1} [color={2}];",
                                    actor.client,
                                    output.split("::").last().unwrap(),
                                    color
                                ),
                            }
                        })
                        .collect::<Vec<String>>()
                })
            })
            .flatten()
            .collect();
        let inputs: Vec<_> = self
            .actors
            .iter()
            .filter_map(|actor| {
                actor.inputs.as_ref().map(|inputs| {
                    inputs
                        .iter()
                        .map(|input| {
                            let color = lookup
                                .entry(actor.inputs_rate)
                                .or_insert_with(|| colors.next().unwrap());
                            format!(
                                r#"{0} -> {1} [label="{0}", color={2}];"#,
                                input.split("::").last().unwrap(),
                                actor.client,
                                color
                            )
                        })
                        .collect::<Vec<String>>()
                })
            })
            .flatten()
            .collect();
        format!(
            r#"
digraph  G {{
  overlap = scale;
  splines = true;
  bgcolor = gray24;
  {{node [shape=box, width=1.5, style="rounded,filled", fillcolor=lightgray]; {};}}
  node [shape=point, fillcolor=gray24, color=lightgray];

  /* Outputs */
{{
  edge [arrowhead=none,colorscheme=dark28];
  {}
}}
{{
  /* Inputs */
  edge [arrowhead=vee,fontsize=9, fontcolor=lightgray, labelfloat=true,colorscheme=dark28]
  {}
}}
}}
"#,
            self.actors
                .iter()
                .map(|actor| actor.client.as_str())
                .collect::<Vec<&str>>()
                .join("; "),
            outputs.join("\n"),
            inputs.join("\n"),
        )
    }
    /// Writes the output of [Graph::to_string()] to a file
    pub fn to_dot<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(path)?;
        write!(&mut file, "{}", self.to_string())?;
        Ok(())
    }
}
