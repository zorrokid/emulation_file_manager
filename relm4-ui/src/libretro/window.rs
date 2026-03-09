use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        glib::{self},
        prelude::*,
    },
};

use libretro_runner::{core::LibretroCore, frame_buffer::FrameBuffer};

use super::input::map_key_event;

impl std::fmt::Debug for LibretroWindowModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibretroWindowModel")
            .finish_non_exhaustive()
    }
}

pub struct LibretroWindowModel {
    /// The active core, wrapped in Rc<RefCell<Option<_>>> so the game loop
    /// timer closure and key controller closures (which must be 'static) can
    /// share ownership without cloning.
    ///
    /// Rc rather than Arc: LibretroCore is !Send (cpal::Stream is not Send),
    /// and all accesses happen on the GTK main thread — glib::timeout_add_local
    /// and GTK signal handlers never leave the main thread.
    ///
    /// None before the first Launch and after each Close.
    core: Rc<RefCell<Option<LibretroCore>>>,

    /// Stored so we can call queue_draw() from the timer closure and
    /// set_draw_func() when a new core is loaded.
    drawing_area: gtk::DrawingArea,

    /// The glib timer driving the game loop. Stored so we can remove it
    /// on Close before calling core.shutdown() — retro_run() must not be
    /// called after retro_deinit().
    timer_source_id: Option<glib::SourceId>,

    /// Temp files to clean up after the session ends. Populated on Launch,
    /// drained and returned to the parent via SessionEnded on Close.
    temp_files: Vec<String>,
}

#[derive(Debug)]
pub enum LibretroWindowMsg {
    Launch {
        core_path: PathBuf,
        rom_path: PathBuf,
        system_dir: PathBuf,
        /// Temp files extracted during ROM preparation — passed back to the
        /// parent via SessionEnded so it can call cleanup().
        temp_files: Vec<String>,
    },
    Close,
}

#[derive(Debug)]
pub enum LibretroWindowOutput {
    Error(String),
    /// Emitted on Close. The parent should call LibretroRunnerService::cleanup_files()
    /// with these file names to remove the extracted temp ROM files.
    SessionEnded(Vec<String>),
}

#[relm4::component(pub)]
impl Component for LibretroWindowModel {
    type Input = LibretroWindowMsg;
    type Output = LibretroWindowOutput;
    type CommandOutput = ();
    type Init = ();

    view! {
        gtk::Window {
            // TODO: Show system and software title
            set_title: Some("Libretro"),
            set_default_size: (640, 480),
            set_resizable: true,

            connect_close_request[sender] => move |_| {
                tracing::info!("Closing libretro window");
                sender.input(LibretroWindowMsg::Close);
                // Return Stop to prevent GTK from destroying the window.
                // We hide it instead so it can be reused for a later launch.
                glib::Propagation::Stop
            },

            // #[local_ref] tells relm4 to use the `drawing_area` variable
            // that already exists in the local scope rather than creating a
            // new widget. The `-> gtk::DrawingArea` part is a type hint.
            #[local_ref]
            drawing_area -> gtk::DrawingArea {
                set_hexpand: true,
                set_vexpand: true,
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        tracing::info!("Initializing libretro window component");
        let model = LibretroWindowModel {
            core: Rc::new(RefCell::new(None)),
            drawing_area: gtk::DrawingArea::new(),
            timer_source_id: None,
            temp_files: Vec::new(),
        };

        // Create the key controller once and attach it to the window here in
        // init().
        //
        // The handler closes over the core Rc. When no core is loaded (between
        // sessions) the borrow returns None and the key event is silently ignored.
        let key_controller = gtk::EventControllerKey::new();

        let core_pressed = Rc::clone(&model.core);
        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            let core = core_pressed.borrow();
            if let Some(core) = core.as_ref() {
                map_key_event(keyval, &core.input_state, true);
            }
            // Stop propagation so the key doesn't trigger other GTK actions.
            glib::Propagation::Stop
        });

        let core_released = Rc::clone(&model.core);
        key_controller.connect_key_released(move |_, keyval, _, _| {
            let core = core_released.borrow();
            if let Some(core) = core.as_ref() {
                map_key_event(keyval, &core.input_state, false);
            }
        });

        root.add_controller(key_controller);

        // Provide the local binding that the #[local_ref] in view! expects.
        let drawing_area = &model.drawing_area;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            LibretroWindowMsg::Launch {
                core_path,
                rom_path,
                system_dir,
                temp_files,
            } => {
                tracing::info!(
                    core_path = ?core_path,
                    rom_path = ?rom_path,
                    system_dir = ?system_dir,
                    temp_files = ?temp_files,
                    "Launching libretro session",
                );
                self.temp_files = temp_files;

                match LibretroCore::load(&core_path, &rom_path, &system_dir) {
                    Ok(core) => {
                        tracing::info!("Libretro core loaded successfully");
                        let fps = core.fps;
                        tracing::info!(fps, "Core reports FPS");

                        // Clone the frame_buffer Arc before moving core into the
                        // RefCell, so we can set up the draw func without locking.
                        let frame_buffer = Arc::clone(&core.frame_buffer);

                        *self.core.borrow_mut() = Some(core);
                        tracing::info!("Libretro core stored in model");

                        self.setup_draw_func(frame_buffer);
                        self.start_game_loop(fps);

                        root.present();
                    }
                    Err(e) => {
                        // Emit SessionEnded even on load failure so the parent
                        // calls cleanup_files() on the already-extracted temp files.
                        let files = std::mem::take(&mut self.temp_files);
                        sender
                            .output(LibretroWindowOutput::SessionEnded(files))
                            .ok();
                        sender
                            .output(LibretroWindowOutput::Error(e.to_string()))
                            .ok();
                    }
                }
            }

            LibretroWindowMsg::Close => {
                tracing::info!("Closing libretro session");
                // Stop the timer first — retro_run() must not be called
                // after retro_deinit() which shutdown() will call.
                self.stop_game_loop();

                // Take the core out of the RefCell before shutting down,
                // so the borrow is released before the potentially slow deinit.
                let core = self.core.borrow_mut().take();
                if let Some(core) = core {
                    tracing::info!("Shutting down libretro core");
                    core.shutdown();
                }

                // Return temp file names to the parent for cleanup.
                // std::mem::take() moves the Vec out, leaving an empty Vec in its place.
                let files = std::mem::take(&mut self.temp_files);
                sender
                    .output(LibretroWindowOutput::SessionEnded(files))
                    .ok();

                tracing::info!("Hiding libretro window");
                root.hide();
            }
        }
    }
}

impl LibretroWindowModel {
    /// Install a cairo draw function that reads pixel data from the shared
    /// frame buffer and paints it scaled to fit the drawing area.
    /// Called once after a successful core load.
    fn setup_draw_func(&self, frame_buffer: Arc<Mutex<FrameBuffer>>) {
        tracing::info!("Setting up drawing function for libretro window");
        self.drawing_area
            .set_draw_func(move |_da, cr, widget_width, widget_height| {
                let fb = frame_buffer.lock().expect("frame buffer lock");

                if fb.rgba_data.is_empty() || fb.width == 0 || fb.height == 0 {
                    return;
                }

                // Skip rendering if widget size is zero (e.g., during window resize to minimum).
                // Otherwise cr.scale(0.0, 0.0) creates an invalid matrix and panics.
                if widget_width == 0 || widget_height == 0 {
                    return;
                }

                // Clone the pixel data out before dropping the lock so we
                // don't hold it during the cairo surface creation.
                let data = fb.rgba_data.clone();
                let fb_width = fb.width as i32;
                let fb_height = fb.height as i32;
                drop(fb);

                // Wrap the pixel bytes in a cairo ImageSurface.
                // ARgb32 matches our stored format: [B, G, R, A] per pixel on
                // little-endian x86_64. Stride = 4 bytes * width (no row padding).
                let surface = gtk::cairo::ImageSurface::create_for_data(
                    data,
                    gtk::cairo::Format::ARgb32,
                    fb_width,
                    fb_height,
                    fb_width * 4,
                )
                .expect("cairo ImageSurface");

                // Scale uniformly to fit the widget, preserving aspect ratio.
                let scale = (widget_width as f64 / fb_width as f64)
                    .min(widget_height as f64 / fb_height as f64);

                // Centre the scaled image in the widget.
                let offset_x = (widget_width as f64 - fb_width as f64 * scale) / 2.0;
                let offset_y = (widget_height as f64 - fb_height as f64 * scale) / 2.0;

                cr.translate(offset_x, offset_y);
                cr.scale(scale, scale);
                cr.set_source_surface(&surface, 0.0, 0.0)
                    .expect("set source surface");

                // Source operator ignores alpha and draws the surface as-is,
                // avoiding any blending with what's behind the widget.
                cr.set_operator(gtk::cairo::Operator::Source);
                cr.paint().expect("cairo paint");
            });
    }

    /// Start a glib main-loop timer that calls retro_run() at the core's
    /// native frame rate.
    ///
    /// glib::timeout_add_local guarantees the closure runs on the main thread
    /// — the same thread that called retro_init() — satisfying the libretro
    /// threading requirement.
    fn start_game_loop(&mut self, fps: f64) {
        tracing::info!(fps = fps, "Starting game loop timer");
        let interval = Duration::from_millis((1000.0 / fps).round() as u64);

        // Clone the Rc so the 'static closure can share ownership of the core.
        let core_ref = Rc::clone(&self.core);
        let drawing_area = self.drawing_area.clone();

        let source_id = glib::timeout_add_local(interval, move || {
            let guard = core_ref.borrow();
            match guard.as_ref() {
                Some(core) => {
                    core.run_frame();
                    // Drop the lock before queue_draw to keep the critical
                    // section as short as possible.
                    drop(guard);
                    // Tell GTK to call our draw function on the next paint cycle.
                    drawing_area.queue_draw();
                    glib::ControlFlow::Continue
                }
                // Core was taken out (shutdown) — stop firing the timer.
                None => glib::ControlFlow::Break,
            }
        });

        self.timer_source_id = Some(source_id);
    }

    /// Remove the game loop timer. Must be called before core.shutdown().
    fn stop_game_loop(&mut self) {
        if let Some(id) = self.timer_source_id.take() {
            tracing::info!("Stopping game loop timer");
            // remove() deregisters the source from the glib main loop.
            id.remove();
        }
    }
}
