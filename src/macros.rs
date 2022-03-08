#[macro_export]
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
    }
#[macro_export]
/// Actor's stage
///
/// The macro returns a [tuple] of [Actor](crate::Actor)s , the first and last [Actor](crate::Actor)s
/// must be an [Initiator](crate::Initiator) and a [Terminator](crate::Terminator)
///
/// # Examples
/// A source, 2 actors and a sink all using the same data type: `Vec<f64>`
///```
/// let (mut source, mut actor1, mut actor2, sink) = stage!(Vec<f64>: src >> a1, a2 << sink)
///```
/// A source, 2 actors and a sink all using the same data type: `Vec<f64>`,
/// the `source` is decimated by a factor 10 and `actor1` upsamples the
/// `source` stream by a factor 10
///```
/// let (mut source, mut actor1, mut actor2, sink) = stage!(Vec<f64>: (src[10] => a1) a2 << sink)
///```
/// A source, 2 actors and a sink all using the same data type: `Vec<f64>`,
/// the `actor1` is decimated by a factor 10 and `actor2` upsamples the
/// `actor1` stream by a factor 10
///```
/// let (mut source, mut actor1, mut actor2, sink) = stage!(Vec<f64>: src >> (a1[10] => a2) << sink)
///```
macro_rules! stage {
        ($data:ty: $initiator:ident >> $($actor:ident),* $(($a1:ident[$rate:ty] => $a2:ident)),* << $terminator:ident ) => {
            (
                Initiator::<$data, 1>::build().tag(stringify!($initiator)),
		$(Actor::<$data, $data, 1, 1>::new().tag(stringify!($actor)),)*
		$(
		    Actor::<$data, $data, 1, $rate>::new().tag(stringify!($a1)),
		    Actor::<$data, $data, $rate, 1>::new().tag(stringify!($a2)),
		)*
                Terminator::<$data, 1>::build().tag(stringify!($terminator)),
            )
        };
        ($data:ty: ($initiator:ident[$irate:ty] => $sampler:ident), $($actor:ident),* $(($a1:ident[$rate:ty] => $a2:ident)),* << $terminator:ident ) => {
            (
                Initiator::<$data, $irate>::build().tag(stringify!($initiator)),
		Actor::<$data, $data, $irate, 1>::new().tag(stringify!($sampler)),
		$(Actor::<$data, $data, 1, 1>::new().tag(stringify!($actor)),)*
		$(
		    Actor::<$data, $data, 1, $rate>::new().tag(stringify!($a1)),
		    Actor::<$data, $data, $rate, 1>::new().tag(stringify!($a2)),
		)*
                Terminator::<$data, 1>::build().tag(stringify!($terminator)),
            )
        };
    }
#[macro_export]
/// Creates input/output channels between pairs of actors
///
/// # Examples
/// Creates a single channel
/// ```
/// channel![actor1 => actor2]
/// ```
/// Creates three channels for the pairs (actor1,actor2), (actor2,actor3) and (actor3,actor4)
/// ```
/// channel![actor1 => actor2  => actor3  => actor4]
/// ```
/// Creates 3 channels between the same pair of actors
/// ```
/// channel![actor1 => actor2; 3]
/// ```
/// Creates 2 channels between a single input and 2 outputs of 2 different actors
/// ```
/// channel![actor1(2) => (actor2, actor3)]
/// ```
macro_rules! channel [
	() => {};
	($from:ident => $to:ident) => {
            dos_actors::one_to_many(&mut $from, &mut [&mut $to]);
	};
	($from:ident => $to:ident; $n:expr) => {
	  (0..$n).for_each(|_| {
              dos_actors::one_to_many(&mut $from, &mut [&mut $to]);})
	};
	($from:ident => $to:ident $(=> $tail:ident)*) => {
            dos_actors::one_to_many(&mut $from, &mut [&mut $to]);
	    channel!($to $(=> $tail)*)
	};
	($from:ident => $to:ident $(=> $tail:ident)*; $n:expr) => {
	  (0..$n).for_each(|_| {
              dos_actors::one_to_many(&mut $from, &mut [&mut $to]);
	      channel!($to $(=> $tail)*)})
	};
	($from:ident($no:expr) => ($($to:ident),+)) => {
	    let inputs = one_to_any(&mut $from, $no);
	    $(let inputs = inputs.and_then(|inputs| inputs.any(&mut[&mut $to]));)+
	};
	($from:ident => ($($to:ident),+)) => {
	    let no: usize = count!($($to)+);
	    let inputs = one_to_any(&mut $from, no);
	    $(let inputs = inputs.and_then(|inputs| inputs.any(&mut[&mut $to]));)+
	};
	($from:ident => ($($to:ident),+); $n:expr) => {
	    let no: usize = count!($($to)+);
	    (0..$n).for_each(|_| {
		let inputs = one_to_any(&mut $from, no);
		$(let inputs = inputs.and_then(|inputs| inputs.any(&mut[&mut $to]));)+
	  })
	};
    ];
#[macro_export]
/// Starts an actor loop with an associated client
///
/// # Examples
/// ```
/// run!(actor, client)
/// ```
macro_rules! run {
    ($actor:expr) => {
        if let Err(e) = $actor.run().await {
            dos_actors::print_error(format!("{} loop ended", $actor.who()), &e);
        };
    };
}
#[macro_export]
/// Spawns actors loop with associated clients
///
/// # Example
/// ```
/// spawn!(actor1, actor2, ...)
/// ```
macro_rules! spawn {
    ($($actor:expr),+) => {
	$(
        tokio::spawn(async move {
	    run!($actor);
        });)+
    };
}
#[macro_export]
/// Same as [crate::spawn] but [crate::Actor::bootstrap] the actor before [crate::Actor::run]ning
macro_rules! spawn_bootstrap {
    ($($actor:ident::<$t:ty,$u:ty>),+) => {
	$(
        tokio::spawn(async move {
            if let Err(e) = $actor.bootstrap::<$t,$u>().await {
		dos_actors::print_error(format!("{} distribute ended", $actor.who()), &e);
            }
	    run!($actor);
        });)+
    };
    ($($actor:ident::$((<$t:ty,$u:ty>)),+),+) => {
	$(
            tokio::spawn(async move {
		$(
		    if let Err(e) = $actor.bootstrap::<$t,$u>().await {
			dos_actors::print_error(format!("{} distribute ended", $actor.who()), &e);
		    }
		)+
		    run!($actor);
        });)+
    };
}
