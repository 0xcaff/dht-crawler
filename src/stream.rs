use futures::{
    Future,
    Stream,
};

pub fn run_forever<S: Stream<Item = (), Error = ()>>(
    stream: S,
) -> impl Future<Item = (), Error = ()> {
    stream
        .skip_while(|_| Ok(true))
        .into_future()
        .map(|_| ())
        .map_err(|_| ())
}
