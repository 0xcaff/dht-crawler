use std::ops::{Generator, GeneratorState};
use std::pin::Pin;

pub struct GeneratorToIterator<G>(G);

impl<G> Iterator for GeneratorToIterator<G>
    where
        G: Generator<Return = ()>,
{
    type Item = G::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        let me = unsafe { Pin::new_unchecked(&mut self.0) };
        match me.resume() {
            GeneratorState::Yielded(x) => Some(x),
            GeneratorState::Complete(_) => None,
        }
    }
}

