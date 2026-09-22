// This module handles all error related logic
// We need to decide if we want error reporting to be done immediately or defered
// If it is to be handled immediately, then error reporting can be though of as as a side effect
// We construct a "world" type that is the owner of error reporting output interface
// this type then has the method for creating the error variant for our result type
// this way we can only construct the error type by reporting an error to the output interface
//
// The other less ergonomic, but easier to implement way is to simply collect all errors
// and then emit them all in one go at the end of each stage
//
// For now, we will go with the second option
// 
// Note: A possible api would be something like passing down a `&mut Output<E: Diagnostic>` that has a `Output::emit(E, E::Ctx)`
// Possibly it could even return something that signals wether or not to continue further

use std::fmt::Display;

pub struct Diagnostics<E>(Vec<E>);

impl<E> Diagnostics<E> {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn push(&mut self, e: E) {
        self.0.push(e);
    }
}

impl<E> Default for Diagnostics<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Display> Display for Diagnostics<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for e in &self.0 {
            writeln!(f, "{}", e)?;
        }
        Ok(())
    }
}
