use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::{fmt, iter};

use dpi::PhysicalSize;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, KeyboardEvent, Navigator, PageTransitionEvent, PointerEvent, WheelEvent};
use web_time::{Duration, Instant};
use winit_core::application::ApplicationHandler;
use winit_core::event::{
    DeviceEvent, DeviceId, ElementState, RawKeyEvent, StartCause, WindowEvent,
};
use winit_core::event_loop::{ControlFlow, DeviceEvents};
use winit_core::window::WindowId;

use super::proxy::EventLoopProxy;
use super::state::State;
use crate::backend::{EventListenerHandle, SafeAreaHandle};
use crate::event_loop::ActiveEventLoop;
use crate::main_thread::MainThreadMarker;
use crate::monitor::MonitorHandler;
use crate::r#async::DispatchRunner;
use crate::web_sys::event::mouse_button_to_id;
use crate::window::Inner;
use crate::{backend, event, EventLoop, PollStrategy, WaitUntilStrategy};

#[derive(Debug)]
pub struct Shared(Rc<Execution>);

impl Clone for Shared {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

type OnEventHandle<T> = RefCell<Option<EventListenerHandle<dyn FnMut(T)>>>;

struct Execution {
    main_thread: MainThreadMarker,
    event_loop_proxy: Arc<EventLoopProxy>,
    control_flow: Cell<ControlFlow>,
    poll_strategy: Cell<PollStrategy>,
    wait_until_strategy: Cell<WaitUntilStrategy>,
    exit: Cell<bool>,
    runner: RefCell<RunnerEnum>,
    suspended: Cell<bool>,
    event_loop_recreation: Cell<bool>,
    events: RefCell<VecDeque<Event>>,
    id: Cell<usize>,
    window: web_sys::Window,
    navigator: Navigator,
    document: Document,
    #[allow(clippy::type_complexity)]
    all_canvases: RefCell<Vec<(WindowId, Weak<backend::Canvas>, DispatchRunner<Inner>)>>,
    redraw_pending: RefCell<HashSet<WindowId>>,
    destroy_pending: RefCell<VecDeque<WindowId>>,
    pub(crate) monitor: Rc<MonitorHandler>,
    safe_area: Rc<SafeAreaHandle>,
    page_transition_event_handle: RefCell<Option<backend::PageTransitionEventHandle>>,
    device_events: Cell<DeviceEvents>,
    on_mouse_move: OnEventHandle<PointerEvent>,
    on_wheel: OnEventHandle<WheelEvent>,
    on_mouse_press: OnEventHandle<PointerEvent>,
    on_mouse_release: OnEventHandle<PointerEvent>,
    on_key_press: OnEventHandle<KeyboardEvent>,
    on_key_release: OnEventHandle<KeyboardEvent>,
    on_visibility_change: OnEventHandle<web_sys::Event>,
}

impl fmt::Debug for Execution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Execution").finish_non_exhaustive()
    }
}

enum RunnerEnum {
    /// The `EventLoop` is created but not being run.
    Pending,
    /// The `EventLoop` is running some async initialization and is waiting to be started.
    Initializing(Runner),
    /// The `EventLoop` is being run.
    Running(Runner),
    /// The `EventLoop` is exited after being started with `EventLoop::run_app`. Since
    /// `EventLoop::run_app` takes ownership of the `EventLoop`, we can be certain
    /// that this event loop will never be run again.
    Destroyed,
}

impl RunnerEnum {
    fn maybe_runner(&self) -> Option<&Runner> {
        match self {
            RunnerEnum::Running(runner) => Some(runner),
            _ => None,
        }
    }
}

struct Runner {
    state: State,
    app: Box<dyn ApplicationHandler>,
    event_loop: ActiveEventLoop,
}

impl fmt::Debug for Runner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Runner")
            .field("state", &self.state)
            .field("app", &"<ApplicationHandler>")
            .field("event_loop", &self.event_loop)
            .finish()
    }
}

impl Runner {
    pub fn new(app: Box<dyn ApplicationHandler>, event_loop: ActiveEventLoop) -> Self {
        Runner { state: State::Init, app, event_loop }
    }

    /// Returns the corresponding `StartCause` for the current `state`, or `None`
    /// when in `Exit` state.
    fn maybe_start_cause(&self) -> Option<StartCause> {
        Some(match self.state {
            State::Init => StartCause::Init,
            State::Poll { .. } => StartCause::Poll,
            State::Wait { start } => StartCause::WaitCancelled { start, requested_resume: None },
            State::WaitUntil { start, end, .. } => {
                StartCause::WaitCancelled { start, requested_resume: Some(end) }
            },
            State::Exit => return None,
        })
    }

    fn handle_single_event(&mut self, runner: &Shared, event: Event) {
        match event {
            Event::NewEvents(cause) => self.app.new_events(&self.event_loop, cause),
            Event::WindowEvent { window_id, event } => {
                self.app.window_event(&self.event_loop, window_id, event)
            },
            Event::ScaleChange { canvas, size, scale } => {
                if let Some(canvas) = canvas.upgrade() {
                    canvas.handle_scale_change(
                        runner,
                        |window_id, event| {
                            self.app.window_event(&self.event_loop, window_id, event);
                        },
                        size,
                        scale,
                    )
                }
            },
            Event::DeviceEvent { device_id, event } => {
                self.app.device_event(&self.event_loop, device_id, event)
            },
            Event::UserWakeUp => self.app.proxy_wake_up(&self.event_loop),
            Event::Suspended => self.app.suspended(&self.event_loop),
            Event::Resumed => self.app.resumed(&self.event_loop),
            Event::CreateSurfaces => self.app.can_create_surfaces(&self.event_loop),
            Event::AboutToWait => self.app.about_to_wait(&self.event_loop),
        }
    }
}

impl Shared {
    pub fn new() -> Self {
        let main_thread = MainThreadMarker::new().expect("only callable from inside the `Window`");
        #[allow(clippy::disallowed_methods)]
        let window = web_sys::window().expect("only callable from inside the `Window`");
        #[allow(clippy::disallowed_methods)]
        let navigator = window.navigator();
        #[allow(clippy::disallowed_methods)]
        let document = window.document().expect("Failed to obtain document");

        Shared(Rc::<Execution>::new_cyclic(|weak| {
            let proxy_spawner = EventLoopProxy::new(main_thread, WeakShared(weak.clone()));

            let monitor = MonitorHandler::new(
                main_thread,
                window.clone(),
                &navigator,
                WeakShared(weak.clone()),
            );

            let safe_area = SafeAreaHandle::new(&window, &document);

            Execution {
                main_thread,
                event_loop_proxy: Arc::new(proxy_spawner),
                control_flow: Cell::new(ControlFlow::default()),
                poll_strategy: Cell::new(PollStrategy::default()),
                wait_until_strategy: Cell::new(WaitUntilStrategy::default()),
                exit: Cell::new(false),
                runner: RefCell::new(RunnerEnum::Pending),
                suspended: Cell::new(false),
                event_loop_recreation: Cell::new(false),
                events: RefCell::new(VecDeque::new()),
                window,
                navigator,
                document,
                id: Cell::new(0),
                all_canvases: RefCell::new(Vec::new()),
                redraw_pending: RefCell::new(HashSet::new()),
                destroy_pending: RefCell::new(VecDeque::new()),
                monitor: Rc::new(monitor),
                safe_area: Rc::new(safe_area),
                page_transition_event_handle: RefCell::new(None),
                device_events: Cell::default(),
                on_mouse_move: RefCell::new(None),
                on_wheel: RefCell::new(None),
                on_mouse_press: RefCell::new(None),
                on_mouse_release: RefCell::new(None),
                on_key_press: RefCell::new(None),
                on_key_release: RefCell::new(None),
                on_visibility_change: RefCell::new(None),
            }
        }))
    }

    pub fn main_thread(&self) -> MainThreadMarker {
        self.0.main_thread
    }

    pub fn window(&self) -> &web_sys::Window {
        &self.0.window
    }

    pub fn navigator(&self) -> &Navigator {
        &self.0.navigator
    }

    pub fn document(&self) -> &Document {
        &self.0.document
    }

    pub fn add_canvas(
        &self,
        id: WindowId,
        canvas: Weak<backend::Canvas>,
        runner: DispatchRunner<Inner>,
    ) {
        self.0.all_canvases.borrow_mut().push((id, canvas, runner));
    }

    pub fn notify_destroy_window(&self, id: WindowId) {
        self.0.destroy_pending.borrow_mut().push_back(id);
    }

    pub(crate) fn start(&self, app: Box<dyn ApplicationHandler>, event_loop: ActiveEventLoop) {
        let mut runner = self.0.runner.borrow_mut();
        assert!(matches!(*runner, RunnerEnum::Pending));
        if self.0.monitor.is_initializing() {
            *runner = RunnerEnum::Initializing(Runner::new(app, event_loop));
        } else {
            *runner = RunnerEnum::Running(Runner::new(app, event_loop));

            drop(runner);

            self.init();
            self.set_listener();
        }
    }

    pub(crate) fn start_delayed(&self) {
        let event_handler = match self.0.runner.replace(RunnerEnum::Pending) {
            RunnerEnum::Initializing(event_handler) => event_handler,
            // The event loop wasn't started yet.
            RunnerEnum::Pending => return,
            _ => unreachable!("event loop already started before waiting for initialization"),
        };
        *self.0.runner.borrow_mut() = RunnerEnum::Running(event_handler);

        self.init();
        self.set_listener();
    }

    // Set the event callback to use for the event loop runner
    // This the event callback is a fairly thin layer over the user-provided callback that closes
    // over a RootActiveEventLoop reference
    fn set_listener(&self) {
        *self.0.page_transition_event_handle.borrow_mut() = Some(backend::on_page_transition(
            self.window().clone(),
            {
                let runner = self.clone();
                move |event: PageTransitionEvent| {
                    if event.persisted() {
                        runner.0.suspended.set(false);
                        runner.send_event(Event::Resumed);
                    }
                }
            },
            {
                let runner = self.clone();
                move |event: PageTransitionEvent| {
                    runner.0.suspended.set(true);
                    if event.persisted() {
                        runner.send_event(Event::Suspended);
                    } else {
                        runner.handle_unload();
                    }
                }
            },
        ));

        let runner = self.clone();
        let window = self.window().clone();
        let navigator = self.navigator().clone();
        *self.0.on_mouse_move.borrow_mut() = Some(EventListenerHandle::new(
            self.window().clone(),
            "pointermove",
            Closure::new(move |event: PointerEvent| {
                if !runner.device_events() {
                    return;
                }

                // chorded button event
                let device_id = event::mkdid(event.pointer_id());

                if let Some(button) = backend::event::mouse_button(&event) {
                    let state = if backend::event::mouse_buttons(&event).contains(button.into()) {
                        ElementState::Pressed
                    } else {
                        ElementState::Released
                    };

                    runner.send_event(Event::DeviceEvent {
                        device_id,
                        event: DeviceEvent::Button {
                            button: mouse_button_to_id(button).into(),
                            state,
                        },
                    });

                    return;
                }

                // pointer move event
                let mut delta = backend::event::MouseDelta::init(&navigator, &event);
                runner.send_events(backend::event::pointer_move_event(event).map(|event| {
                    let delta = delta.delta(&event).to_physical(backend::scale_factor(&window));

                    Event::DeviceEvent {
                        device_id,
                        event: DeviceEvent::PointerMotion { delta: (delta.x, delta.y) },
                    }
                }));
            }),
        ));
        let runner = self.clone();
        let window = self.window().clone();
        *self.0.on_wheel.borrow_mut() = Some(EventListenerHandle::new(
            self.window().clone(),
            "wheel",
            Closure::new(move |event: WheelEvent| {
                if !runner.device_events() {
                    return;
                }

                if let Some(delta) = backend::event::mouse_scroll_delta(&window, &event) {
                    runner.send_event(Event::DeviceEvent {
                        device_id: None,
                        event: DeviceEvent::MouseWheel { delta },
                    });
                }
            }),
        ));
        let runner = self.clone();
        *self.0.on_mouse_press.borrow_mut() = Some(EventListenerHandle::new(
            self.window().clone(),
            "pointerdown",
            Closure::new(move |event: PointerEvent| {
                if !runner.device_events() {
                    return;
                }

                let button = backend::event::mouse_button(&event).expect("no mouse button pressed");
                runner.send_event(Event::DeviceEvent {
                    device_id: event::mkdid(event.pointer_id()),
                    event: DeviceEvent::Button {
                        button: mouse_button_to_id(button).into(),
                        state: ElementState::Pressed,
                    },
                });
            }),
        ));
        let runner = self.clone();
        *self.0.on_mouse_release.borrow_mut() = Some(EventListenerHandle::new(
            self.window().clone(),
            "pointerup",
            Closure::new(move |event: PointerEvent| {
                if !runner.device_events() {
                    return;
                }

                let button = backend::event::mouse_button(&event).expect("no mouse button pressed");
                runner.send_event(Event::DeviceEvent {
                    device_id: event::mkdid(event.pointer_id()),
                    event: DeviceEvent::Button {
                        button: mouse_button_to_id(button).into(),
                        state: ElementState::Released,
                    },
                });
            }),
        ));
        let runner = self.clone();
        *self.0.on_key_press.borrow_mut() = Some(EventListenerHandle::new(
            self.window().clone(),
            "keydown",
            Closure::new(move |event: KeyboardEvent| {
                if !runner.device_events() {
                    return;
                }

                runner.send_event(Event::DeviceEvent {
                    device_id: None,
                    event: DeviceEvent::Key(RawKeyEvent {
                        physical_key: backend::event::key_code(&event),
                        state: ElementState::Pressed,
                    }),
                });
            }),
        ));
        let runner = self.clone();
        *self.0.on_key_release.borrow_mut() = Some(EventListenerHandle::new(
            self.window().clone(),
            "keyup",
            Closure::new(move |event: KeyboardEvent| {
                if !runner.device_events() {
                    return;
                }

                runner.send_event(Event::DeviceEvent {
                    device_id: None,
                    event: DeviceEvent::Key(RawKeyEvent {
                        physical_key: backend::event::key_code(&event),
                        state: ElementState::Released,
                    }),
                });
            }),
        ));
        let runner = self.clone();
        *self.0.on_visibility_change.borrow_mut() = Some(EventListenerHandle::new(
            // Safari <14 doesn't support the `visibilitychange` event on `Window`.
            self.document().clone(),
            "visibilitychange",
            Closure::new(move |_| {
                if !runner.0.suspended.get() {
                    for (id, canvas, _) in &*runner.0.all_canvases.borrow() {
                        if let Some(canvas) = canvas.upgrade() {
                            let is_visible = backend::is_visible(runner.document());
                            // only fire if:
                            // - not visible and intersects
                            // - not visible and we don't know if it intersects yet
                            // - visible and intersects
                            if let (false, Some(true) | None) | (true, Some(true)) =
                                (is_visible, canvas.is_intersecting.get())
                            {
                                runner.send_event(Event::WindowEvent {
                                    window_id: *id,
                                    event: WindowEvent::Occluded(!is_visible),
                                });
                            }
                        }
                    }
                }
            }),
        ));
    }

    // Generate a strictly increasing ID
    // This is used to differentiate windows when handling events
    pub fn generate_id(&self) -> usize {
        let id = self.0.id.get();
        self.0.id.set(id.checked_add(1).expect("exhausted `WindowId`"));

        id
    }

    pub fn request_redraw(&self, id: WindowId) {
        self.0.redraw_pending.borrow_mut().insert(id);
        self.send_events([]);
    }

    fn init(&self) {
        // NB: For consistency all platforms must call `can_create_surfaces` even though Web
        // applications don't themselves have a formal surface destroy/create lifecycle.
        self.run_until_cleared(
            [Event::NewEvents(StartCause::Init), Event::CreateSurfaces].into_iter(),
        );
    }

    // Run the polling logic for the Poll ControlFlow, which involves clearing the queue
    pub fn poll(&self) {
        let start_cause = Event::NewEvents(StartCause::Poll);
        self.run_until_cleared(iter::once(start_cause));
    }

    // Run the logic for waking from a WaitUntil, which involves clearing the queue
    // Generally there shouldn't be events built up when this is called
    pub fn resume_time_reached(&self, start: Instant, requested_resume: Instant) {
        let start_cause =
            Event::NewEvents(StartCause::ResumeTimeReached { start, requested_resume });
        self.run_until_cleared(iter::once(start_cause));
    }

    // Add an event to the event loop runner, from the user or an event handler
    //
    // It will determine if the event should be immediately sent to the user or buffered for later
    pub(crate) fn send_event(&self, event: Event) {
        self.send_events(iter::once(event));
    }

    // Add a user event to the event loop runner.
    //
    // This will schedule the event loop to wake up instead of waking it up immediately if its not
    // running.
    pub(crate) fn send_proxy_wake_up(&self, local: bool) {
        // If the event loop is closed, it should discard any new events
        if self.is_closed() {
            return;
        }

        if local {
            // If the loop is not running and triggered locally, queue on next microtick.
            if let Ok(RunnerEnum::Running(_)) =
                self.0.runner.try_borrow().as_ref().map(Deref::deref)
            {
                self.window().queue_microtask(
                    &Closure::once_into_js({
                        let this = Rc::downgrade(&self.0);
                        move || {
                            if let Some(shared) = this.upgrade() {
                                Shared(shared).send_event(Event::UserWakeUp)
                            }
                        }
                    })
                    .unchecked_into(),
                );

                return;
            }
        }

        self.send_event(Event::UserWakeUp);
    }

    // Add a series of events to the event loop runner
    //
    // It will determine if the event should be immediately sent to the user or buffered for later
    pub(crate) fn send_events(&self, events: impl IntoIterator<Item = Event>) {
        // If the event loop is closed, it should discard any new events
        if self.is_closed() {
            return;
        }
        // If we can run the event processing right now, or need to queue this and wait for later
        let mut process_immediately = true;
        match self.0.runner.try_borrow().as_ref().map(Deref::deref) {
            // If the runner is attached but not running, we always wake it up.
            Ok(RunnerEnum::Running(_)) => (),
            // The runner still hasn't been attached: queue this event and wait for it to be
            Ok(RunnerEnum::Pending | RunnerEnum::Initializing(_)) => {
                process_immediately = false;
            },
            // Some other code is mutating the runner, which most likely means
            // the event loop is running and busy. So we queue this event for
            // it to be processed later.
            Err(_) => {
                process_immediately = false;
            },
            // This is unreachable since `self.is_closed() == true`.
            Ok(RunnerEnum::Destroyed) => unreachable!(),
        }
        if !process_immediately {
            // Queue these events to look at later
            self.0.events.borrow_mut().extend(events);
            return;
        }
        // At this point, we know this is a fresh set of events
        // Now we determine why new events are incoming, and handle the events
        let start_cause = match (self.0.runner.borrow().maybe_runner())
            .unwrap_or_else(|| {
                unreachable!("The runner cannot process events when it is not attached")
            })
            .maybe_start_cause()
        {
            Some(c) => c,
            // If we're in the exit state, don't do event processing
            None => return,
        };
        // Take the start event, then the events provided to this function, and run an iteration of
        // the event loop
        let start_event = Event::NewEvents(start_cause);
        let events = iter::once(start_event).chain(events);
        self.run_until_cleared(events);
    }

    // Process the destroy-pending windows. This should only be called from
    // `run_until_cleared`, somewhere between emitting `NewEvents` and `AboutToWait`.
    fn process_destroy_pending_windows(&self) {
        while let Some(id) = self.0.destroy_pending.borrow_mut().pop_front() {
            self.0.all_canvases.borrow_mut().retain(|&(item_id, ..)| item_id != id);
            self.handle_event(Event::WindowEvent {
                window_id: id,
                event: winit_core::event::WindowEvent::Destroyed,
            });
            self.0.redraw_pending.borrow_mut().remove(&id);
        }
    }

    // Given the set of new events, run the event loop until the main events and redraw events are
    // cleared
    //
    // This will also process any events that have been queued or that are queued during processing
    fn run_until_cleared(&self, events: impl Iterator<Item = Event>) {
        for event in events {
            self.handle_event(event);
        }
        self.process_destroy_pending_windows();

        // Collect all of the redraw events to avoid double-locking the RefCell
        let redraw_events: Vec<WindowId> = self.0.redraw_pending.borrow_mut().drain().collect();
        for window_id in redraw_events {
            self.handle_event(Event::WindowEvent {
                window_id,
                event: WindowEvent::RedrawRequested,
            });
        }

        self.handle_event(Event::AboutToWait);

        self.apply_control_flow();
        // If the event loop is closed, it has been closed this iteration and now the closing
        // event should be emitted
        if self.is_closed() {
            self.handle_loop_destroyed();
        }
    }

    fn handle_unload(&self) {
        self.exit();
        self.apply_control_flow();
        // We don't call `handle_loop_destroyed` here because we don't need to
        // perform cleanup when the Web browser is going to destroy the page.
        //
        // We do want to run the application handler's `Drop` impl.
        *self.0.runner.borrow_mut() = RunnerEnum::Destroyed;
    }

    // handle_event takes in events and either queues them or applies a callback
    //
    // It should only ever be called from `run_until_cleared`.
    fn handle_event(&self, event: Event) {
        if self.is_closed() {
            self.exit();
        }
        match *self.0.runner.borrow_mut() {
            RunnerEnum::Running(ref mut runner) => {
                runner.handle_single_event(self, event);
            },
            // If an event is being handled without a runner somehow, add it to the event queue so
            // it will eventually be processed
            RunnerEnum::Pending => self.0.events.borrow_mut().push_back(event),
            // If the Runner has been destroyed, there is nothing to do.
            RunnerEnum::Destroyed => return,
            // This function should never be called if we are still waiting for something.
            RunnerEnum::Initializing(_) => unreachable!(),
        }

        let is_closed = self.exiting();

        // Don't take events out of the queue if the loop is closed or the runner doesn't exist
        // If the runner doesn't exist and this method recurses, it will recurse infinitely
        if !is_closed && self.0.runner.borrow().maybe_runner().is_some() {
            // Pre-fetch window commands to avoid having to wait until the next event loop cycle
            // and potentially block other threads in the meantime.
            for (_, window, runner) in self.0.all_canvases.borrow().iter() {
                if let Some(window) = window.upgrade() {
                    runner.run(self.main_thread());
                    drop(window)
                }
            }

            // Take an event out of the queue and handle it
            // Make sure not to let the borrow_mut live during the next handle_event
            let event = {
                let mut events = self.0.events.borrow_mut();

                // Pre-fetch `UserEvent`s to avoid having to wait until the next event loop cycle.
                events.extend(self.0.event_loop_proxy.take().then_some(Event::UserWakeUp));

                events.pop_front()
            };
            if let Some(event) = event {
                self.handle_event(event);
            }
        }
    }

    // Apply the new ControlFlow that has been selected by the user
    // Start any necessary timeouts etc
    fn apply_control_flow(&self) {
        let new_state = if self.exiting() {
            State::Exit
        } else {
            match self.control_flow() {
                ControlFlow::Poll => {
                    let cloned = self.clone();
                    State::Poll {
                        _request: backend::Schedule::new(
                            self.poll_strategy(),
                            self.window(),
                            move || cloned.poll(),
                        ),
                    }
                },
                ControlFlow::Wait => State::Wait { start: Instant::now() },
                ControlFlow::WaitUntil(end) => {
                    let start = Instant::now();

                    let delay = if end <= start { Duration::from_millis(0) } else { end - start };

                    let cloned = self.clone();

                    State::WaitUntil {
                        start,
                        end,
                        _timeout: backend::Schedule::new_with_duration(
                            self.wait_until_strategy(),
                            self.window(),
                            move || cloned.resume_time_reached(start, end),
                            delay,
                        ),
                    }
                },
            }
        };

        if let RunnerEnum::Running(ref mut runner) = *self.0.runner.borrow_mut() {
            runner.state = new_state;
        }
    }

    fn handle_loop_destroyed(&self) {
        let all_canvases = std::mem::take(&mut *self.0.all_canvases.borrow_mut());
        *self.0.page_transition_event_handle.borrow_mut() = None;
        *self.0.on_mouse_move.borrow_mut() = None;
        *self.0.on_wheel.borrow_mut() = None;
        *self.0.on_mouse_press.borrow_mut() = None;
        *self.0.on_mouse_release.borrow_mut() = None;
        *self.0.on_key_press.borrow_mut() = None;
        *self.0.on_key_release.borrow_mut() = None;
        *self.0.on_visibility_change.borrow_mut() = None;
        // Dropping the `Runner` drops the event handler closure, which will in
        // turn drop all `Window`s moved into the closure.
        *self.0.runner.borrow_mut() = RunnerEnum::Destroyed;
        for (_, canvas, _) in all_canvases {
            // In case any remaining `Window`s are still not dropped, we will need
            // to explicitly remove the event handlers associated with their canvases.
            if let Some(canvas) = canvas.upgrade() {
                canvas.remove_listeners();
            }
        }
        // At this point, the `self.0` `Rc` should only be strongly referenced
        // by the following:
        // * `self`, i.e. the item which triggered this event loop wakeup, which is usually a
        //   `wasm-bindgen` `Closure`, which will be dropped after returning to the JS glue code.
        // * The `ActiveEventLoop` leaked inside `EventLoop::run_app` due to the JS exception thrown
        //   at the end.
        // * For each undropped `Window`:
        //     * The `register_redraw_request` closure.
        //     * The `destroy_fn` closure.
        if self.0.event_loop_recreation.get() {
            EventLoop::allow_event_loop_recreation();
        }
    }

    // Check if the event loop is currently closed
    fn is_closed(&self) -> bool {
        match self.0.runner.try_borrow().as_ref().map(Deref::deref) {
            Ok(RunnerEnum::Running(runner)) => runner.state.exiting(),
            // The event loop is not closed since it is not initialized.
            Ok(RunnerEnum::Pending) => false,
            // The event loop is closed since it has been destroyed.
            Ok(RunnerEnum::Destroyed) => true,
            // The event loop is not closed since its still waiting to be started.
            Ok(RunnerEnum::Initializing(_)) => false,
            // Some other code is mutating the runner, which most likely means
            // the event loop is running and busy.
            Err(_) => false,
        }
    }

    pub fn listen_device_events(&self, allowed: DeviceEvents) {
        self.0.device_events.set(allowed)
    }

    fn device_events(&self) -> bool {
        match self.0.device_events.get() {
            DeviceEvents::Always => true,
            DeviceEvents::WhenFocused => {
                self.0.all_canvases.borrow().iter().any(|(_, canvas, _)| {
                    if let Some(canvas) = canvas.upgrade() {
                        canvas.has_focus.get()
                    } else {
                        false
                    }
                })
            },
            DeviceEvents::Never => false,
        }
    }

    pub fn event_loop_recreation(&self, allow: bool) {
        self.0.event_loop_recreation.set(allow)
    }

    pub(crate) fn control_flow(&self) -> ControlFlow {
        self.0.control_flow.get()
    }

    pub(crate) fn set_control_flow(&self, control_flow: ControlFlow) {
        self.0.control_flow.set(control_flow)
    }

    pub(crate) fn exit(&self) {
        self.0.exit.set(true)
    }

    pub(crate) fn exiting(&self) -> bool {
        self.0.exit.get()
    }

    pub(crate) fn set_poll_strategy(&self, strategy: PollStrategy) {
        self.0.poll_strategy.set(strategy)
    }

    pub(crate) fn poll_strategy(&self) -> PollStrategy {
        self.0.poll_strategy.get()
    }

    pub(crate) fn set_wait_until_strategy(&self, strategy: WaitUntilStrategy) {
        self.0.wait_until_strategy.set(strategy)
    }

    pub(crate) fn wait_until_strategy(&self) -> WaitUntilStrategy {
        self.0.wait_until_strategy.get()
    }

    pub(crate) fn event_loop_proxy(&self) -> &Arc<EventLoopProxy> {
        &self.0.event_loop_proxy
    }

    pub(crate) fn weak(&self) -> WeakShared {
        WeakShared(Rc::downgrade(&self.0))
    }

    pub(crate) fn monitor(&self) -> &Rc<MonitorHandler> {
        &self.0.monitor
    }

    pub(crate) fn safe_area(&self) -> &Rc<SafeAreaHandle> {
        &self.0.safe_area
    }
}

#[derive(Clone, Debug)]
pub struct WeakShared(Weak<Execution>);

impl WeakShared {
    pub fn upgrade(&self) -> Option<Shared> {
        self.0.upgrade().map(Shared)
    }
}

#[allow(clippy::enum_variant_names)]
pub(crate) enum Event {
    NewEvents(StartCause),
    WindowEvent { window_id: WindowId, event: WindowEvent },
    ScaleChange { canvas: Weak<backend::Canvas>, size: PhysicalSize<u32>, scale: f64 },
    DeviceEvent { device_id: Option<DeviceId>, event: DeviceEvent },
    Suspended,
    CreateSurfaces,
    Resumed,
    AboutToWait,
    UserWakeUp,
}
