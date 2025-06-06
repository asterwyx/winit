use crate::application::ApplicationHandler;
use crate::error::EventLoopError;
#[cfg(doc)]
use crate::{
    event_loop::{pump_events::EventLoopExtPumpEvents, ActiveEventLoop},
    window::Window,
};

#[allow(rustdoc::broken_intra_doc_links)] // FIXME(madsmtm): Fix these.
/// Additional methods on [`EventLoop`] to return control flow to the caller.
pub trait EventLoopExtRunOnDemand {
    /// Run the application with the event loop on the calling thread.
    ///
    /// Unlike [`EventLoop::run_app`], this function accepts non-`'static` (i.e. non-`move`)
    /// closures and it is possible to return control back to the caller without
    /// consuming the `EventLoop` (by using [`exit()`]) and
    /// so the event loop can be re-run after it has exit.
    ///
    /// It's expected that each run of the loop will be for orthogonal instantiations of your
    /// Winit application, but internally each instantiation may re-use some common window
    /// system resources, such as a display server connection.
    ///
    /// This API is not designed to run an event loop in bursts that you can exit from and return
    /// to while maintaining the full state of your application. (If you need something like this
    /// you can look at the [`EventLoopExtPumpEvents::pump_app_events()`] API)
    ///
    /// Each time `run_app_on_demand` is called the startup sequence of `init`, followed by
    /// `resume` is being preserved.
    ///
    /// See the [`set_control_flow()`] docs on how to change the event loop's behavior.
    ///
    /// # Caveats
    /// - This extension isn't available on all platforms, since it's not always possible to return
    ///   to the caller (specifically this is impossible on iOS and Web - though with the Web
    ///   backend it is possible to use `EventLoopExtWeb::spawn_app()`[^1] more than once instead).
    /// - No [`Window`] state can be carried between separate runs of the event loop.
    ///
    /// You are strongly encouraged to use [`EventLoop::run_app()`] for portability, unless you
    /// specifically need the ability to re-run a single event loop more than once
    ///
    /// # Supported Platforms
    /// - Windows
    /// - Linux
    /// - macOS
    /// - Android
    ///
    /// # Unsupported Platforms
    /// - **Web:**  This API is fundamentally incompatible with the event-based way in which Web
    ///   browsers work because it's not possible to have a long-running external loop that would
    ///   block the browser and there is nothing that can be polled to ask for new events. Events
    ///   are delivered via callbacks based on an event loop that is internal to the browser itself.
    /// - **iOS:** It's not possible to stop and start an `UIApplication` repeatedly on iOS.
    ///
    /// [^1]: `spawn_app()` is only available on the Web platforms.
    ///
    /// [`exit()`]: ActiveEventLoop::exit()
    /// [`set_control_flow()`]: ActiveEventLoop::set_control_flow()
    fn run_app_on_demand<A: ApplicationHandler>(&mut self, app: A) -> Result<(), EventLoopError>;
}
