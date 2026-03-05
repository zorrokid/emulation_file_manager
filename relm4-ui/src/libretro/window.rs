use std::{
    path::PathBuf,
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

use libretro_runner::{core::LibretroCore, frame_buffer::FrameBuffer, input::InputState};

use super::input::map_key_event;

pub struct LibretroWindowModel {
    /// The active core, wrapped in Arc<Mutex<Option<_>>> so the game loop
    /// timer closure (which must be 'static) can share ownership.
    /// None before the first Launch and after each Close.
    core: Arc<Mutex<Option<LibretroCore>>>,

    /// Stored so we can call queue_draw() from the timer closure and
    /// set_draw_func() when a new core is loaded.
    drawing_area: gtk::DrawingArea,

    /// The glib timer driving the game loop. Stored so we can remove it
    /// on Close before calling core.shutdown() — retro_run() must not be
    /// called after retro_deinit().
    timer_source_id: Option<glib::SourceId>,
}

#[derive(Debug)]
pub enum LibretroWindowMsg {
    Launch {
        core_path: PathBuf,
        rom_path: PathBuf,
        system_dir: PathBuf,
    },
    Close,
}

#[derive(Debug)]
pub enum LibretroWindowOutput {
    Error(String),
}

#[relm4::component(pub)]
impl Component for LibretroWindowModel {
    type Input = LibretroWindowMsg;
    type Output = LibretroWindowOutput;
    type CommandOutput = ();
    type Init = ();

    view! {
        gtk::Window {
            set_title: Some("Libretro"),
            set_default_size: (640, 480),
            set_resizable: true,

            connect_close_request[sender] => move |_| {
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
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = LibretroWindowModel {
            core: Arc::new(Mutex::new(None)),
            drawing_area: gtk::DrawingArea::new(),
            timer_source_id: None,
        };

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
            } => {
                match LibretroCore::load(&core_path, &rom_path, &system_dir) {
                    Ok(core) => {
                        let fps = core.fps;
                        // Clone the Arcs before moving core into the Mutex,
                        // so we can set up draw func and input without locking.
                        let frame_buffer = Arc::clone(&core.frame_buffer);
                        let input_state = Arc::clone(&core.input_state);

                        *self.core.lock().expect("core lock") = Some(core);

                        self.setup_draw_func(frame_buffer);
                        self.setup_input(root, input_state);
                        self.start_game_loop(fps);

                        root.present();
                    }
                    Err(e) => {
                        sender.output(LibretroWindowOutput::Error(e.to_string())).ok();
                    }
                }
            }

            LibretroWindowMsg::Close => {
                // Stop the timer first — retro_run() must not be called
                // after retro_deinit() which shutdown() will call.
                self.stop_game_loop();

                // Take the core out of the Mutex before shutting down,
                // so the lock is released before the potentially slow deinit.
                let core = self.core.lock().expect("core lock").take();
                if let Some(core) = core {
                    core.shutdown();
                }

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
        self.drawing_area.set_draw_func(
            move |_da, cr, widget_width, widget_height| {
                let fb = frame_buffer.lock().expect("frame buffer lock");

                if fb.rgba_data.is_empty() || fb.width == 0 || fb.height == 0 {
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
                cr.set_source_surface(&surface, 0.0, 0.0).expect("set source surface");

                // Source operator ignores alpha and draws the surface as-is,
                // avoiding any blending with what's behind the widget.
                cr.set_operator(gtk::cairo::Operator::Source);
                cr.paint().expect("cairo paint");
            },
        );
    }

    /// Add an EventControllerKey to the root window that routes keyboard
    /// events to the libretro input state.
    ///
    /// We add a new controller on each Launch. For the MVP this is harmless
    /// since the window is only launched once per session. If reuse across
    /// multiple sessions is needed, the controller should be stored and reused.
    fn setup_input(&self, root: &gtk::Window, input_state: Arc<Mutex<InputState>>) {
        let key_controller = gtk::EventControllerKey::new();

        let input_pressed = Arc::clone(&input_state);
        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            map_key_event(keyval, &input_pressed, true);
            // Stop propagation so the key doesn't trigger other GTK actions.
            glib::Propagation::Stop
        });

        let input_released = Arc::clone(&input_state);
        key_controller.connect_key_released(move |_, keyval, _, _| {
            map_key_event(keyval, &input_released, false);
        });

        root.add_controller(key_controller);
    }

    /// Start a glib main-loop timer that calls retro_run() at the core's
    /// native frame rate.
    ///
    /// glib::timeout_add_local guarantees the closure runs on the main thread
    /// — the same thread that called retro_init() — satisfying the libretro
    /// threading requirement.
    fn start_game_loop(&mut self, fps: f64) {
        let interval = Duration::from_millis((1000.0 / fps).round() as u64);

        // Clone the Arc so the 'static closure can share ownership of the core.
        let core_ref = Arc::clone(&self.core);
        let drawing_area = self.drawing_area.clone();

        let source_id = glib::timeout_add_local(interval, move || {
            let guard = core_ref.lock().expect("core lock");
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
            // remove() deregisters the source from the glib main loop.
            id.remove();
        }
    }
}
