//! Defines the execution context for algorithms, which can be used to check for cancellation.

/// A trait that can be implemented by any type that wants to provide cancellation functionality.
pub trait Cancellation {
    fn is_cancelled(&self) -> bool;
}

/// The execution context for algorithms, which can be used to check for cancellation.
#[derive(Clone, Copy, Default)]
pub struct ExecutionContext<'a> {
    /// An optional reference to a cancellation object. If `None`, cancellation is not supported.
    cancellation: Option<&'a dyn Cancellation>,
}

impl<'a> ExecutionContext<'a> {
    /// Creates a new execution context with the given cancellation object.
    ///
    /// # Arguments
    /// * `cancellation` - A reference to a cancellation object that implements the `Cancellation` trait.
    pub fn new(cancellation: &'a dyn Cancellation) -> Self {
        Self {
            cancellation: Some(cancellation),
        }
    }

    /// Creates a new execution context with no cancellation support.
    pub fn none() -> Self {
        Self { cancellation: None }
    }

    /// Checks if the execution has been cancelled. If cancellation is not supported, this will always return `false`.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_some_and(|c| c.is_cancelled())
    }
}
