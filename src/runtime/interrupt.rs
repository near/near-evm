use crate::runtime::{ExitFatal, Handler, Runtime};

/// Interrupt resolution.
pub enum Resolve<'a, 'b, 'config, H: Handler> {
    /// Create interrupt resolution.
    Create(H::CreateInterrupt, ResolveCreate<'a, 'b, 'config>),
    /// Call interrupt resolution.
    Call(H::CallInterrupt, ResolveCall<'a, 'b, 'config>),
}

/// Create interrupt resolution.
pub struct ResolveCreate<'a, 'b, 'config> {
    runtime: &'a mut Runtime<'b, 'config>,
}

impl<'a, 'b, 'config> ResolveCreate<'a, 'b, 'config> {
    pub(crate) fn new(runtime: &'a mut Runtime<'b, 'config>) -> Self {
        Self { runtime }
    }
}

impl<'a, 'b, 'config> Drop for ResolveCreate<'a, 'b, 'config> {
    fn drop(&mut self) {
        self.runtime.status = Err(ExitFatal::UnhandledInterrupt.into());
        self.runtime.exit(ExitFatal::UnhandledInterrupt.into());
    }
}

/// Call interrupt resolution.
pub struct ResolveCall<'a, 'b, 'config> {
    runtime: &'a mut Runtime<'b, 'config>,
}

impl<'a, 'b, 'config> ResolveCall<'a, 'b, 'config> {
    pub(crate) fn new(runtime: &'a mut Runtime<'b, 'config>) -> Self {
        Self { runtime }
    }
}

impl<'a, 'b, 'config> Drop for ResolveCall<'a, 'b, 'config> {
    fn drop(&mut self) {
        self.runtime.status = Err(ExitFatal::UnhandledInterrupt.into());
        self.runtime.exit(ExitFatal::UnhandledInterrupt.into());
    }
}
