//! Fluent builder for Runtime construction.
//!
//! Provides a builder pattern interface for creating and configuring Runtime instances.

use crate::runtime::Runtime;

/// Builder for constructing Runtime instances with fluent API.
///
/// Allows customizable runtime instantiation following the builder pattern.
/// Currently creates a basic runtime, but designed to support future configuration options.
///
/// # Example
/// ```ignore
/// let rt = RuntimeBuilder::new().build();
/// ```
pub struct RuntimeBuilder {
    enable_io: bool,
    enable_fs: bool,
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBuilder {
    /// Creates a new runtime builder.
    ///
    /// Initializes a builder instance for constructing a Runtime.
    ///
    /// # Example
    /// ```ignore
    /// let builder = RuntimeBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            enable_io: false,
            enable_fs: false,
        }
    }

    /// Enables reactor-backed I/O support for the runtime being built.
    pub fn enable_io(mut self) -> Self {
        self.enable_io = true;
        self
    }

    /// Enables filesystem support for the runtime being built.
    pub fn enable_fs(mut self) -> Self {
        self.enable_fs = true;
        // Filesystem support relies on reactor I/O for non-blocking operations.
        self.enable_io = true;
        self
    }

    /// Builds and returns a configured Runtime instance.
    ///
    /// Consumes the builder and constructs a Runtime with the current configuration.
    /// Currently creates a default runtime but designed to support configuration options.
    ///
    /// # Returns
    /// A newly constructed Runtime instance
    ///
    /// # Example
    /// ```ignore
    /// let rt = RuntimeBuilder::new().build();
    /// ```
    pub fn build(self) -> Runtime {
        Runtime::with_features(self.enable_io, self.enable_fs)
    }
}
