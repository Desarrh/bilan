#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod constats;
mod modele;
mod sources;
mod tache;

use egui_software_backend::{BufferMutRef, ColorFieldOrder, EguiSoftwareRender};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// Rendu CPU (comme l'explorateur) : pas de contexte GPU, pas des ~150 Mo du pilote.
struct Gfx {
    window: Rc<Window>,
    egui: egui_winit::State,
    _context: softbuffer::Context<Rc<Window>>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    renderer: EguiSoftwareRender,
}

struct Runner {
    ctx: egui::Context,
    gfx: Option<Gfx>,
    app: Option<app::App>,
    settings: Option<app::Settings>,
    next_repaint: Option<Instant>,
    /// Lancé par la tâche planifiée : la fenêtre s'ouvre sans prendre le focus.
    discret: bool,
    started: bool,
    images: u32,
    instance: Option<socle::instance::Instance>,
    /// Un 2e lancement demande de ramener la fenêtre.
    ramener: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Runner {
    fn paint(&mut self, el: &ActiveEventLoop) {
        let (Some(gfx), Some(app)) = (self.gfx.as_mut(), self.app.as_mut()) else { return };
        let size = gfx.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        let mut raw = gfx.egui.take_egui_input(&gfx.window);
        socle::testmode::prepare_input(&mut raw);
        let out = self.ctx.run_ui(raw, |ui| {
            app.tick();
            app.handle_keys();
            app.draw(ui);
        });
        gfx.egui.handle_platform_output(&gfx.window, out.platform_output);
        let prims = self.ctx.tessellate(out.shapes, out.pixels_per_point);
        if gfx.surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buf) = gfx.surface.buffer_mut() else { return };
        buf.fill(app.pal.clear_u32());
        {
            let px: &mut [[u8; 4]] = bytemuck::cast_slice_mut(&mut buf[..]);
            let mut target = BufferMutRef::new(px, w.get() as usize, h.get() as usize);
            gfx.renderer.render(&mut target, &prims, &out.textures_delta, out.pixels_per_point);
        }
        let _ = buf.present();
        if !self.started {
            self.started = true;
            socle::update::confirm_started();
            if std::env::var_os("BILAN_MESURE").is_some() {
                eprintln!("1re image : {} ms", T0.get().map_or(0, |t| t.elapsed().as_millis()));
            }
        }
        // Mode test : `BILAN_CAPTURE=fichier.png` capture la fenêtre invisible une fois chargée, puis quitte.
        if socle::testmode::enabled() && app.pret() {
            self.images += 1;
            if let (Some(f), true) = (std::env::var_os("BILAN_CAPTURE"), self.images > 3) {
                capturer(&gfx.window, std::path::Path::new(&f));
                app.quitter = true;
            } else {
                gfx.window.request_redraw();
            }
        }
        if self.ramener.swap(false, std::sync::atomic::Ordering::Relaxed) && !socle::testmode::enabled() {
            gfx.window.set_minimized(false);
            gfx.window.focus_window();
        }
        if app.quitter {
            self.save();
            el.exit();
        }
    }

    fn save(&mut self) {
        if let Some(app) = self.app.as_mut() {
            let size = self.gfx.as_ref().map(|g| {
                let sz = g.window.inner_size().to_logical::<f32>(g.window.scale_factor());
                (sz.width, sz.height)
            });
            app.flush(size);
        }
    }
}

impl ApplicationHandler<Instant> for Runner {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let (w, h) = self.settings.as_ref().and_then(|s| s.window).unwrap_or((860.0, 900.0));
        let mut attrs = Window::default_attributes()
            .with_title("Bilan")
            .with_inner_size(LogicalSize::new(w.max(520.0), h.max(400.0)))
            .with_min_inner_size(LogicalSize::new(520.0, 400.0))
            .with_window_icon(window_icon());
        let test = socle::testmode::enabled();
        if test {
            attrs = attrs.with_visible(false).with_active(false);
        } else if self.discret {
            attrs = attrs.with_active(false);
        }
        let Ok(window) = el.create_window(attrs) else {
            el.exit();
            return;
        };
        let window = Rc::new(window);
        #[cfg(windows)]
        if test {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(h) = window.window_handle() {
                if let RawWindowHandle::Win32(w) = h.as_raw() {
                    socle::testmode::hide(w.hwnd.get());
                }
            }
        }
        let (Ok(context), egui) = (
            softbuffer::Context::new(window.clone()),
            egui_winit::State::new(self.ctx.clone(), egui::ViewportId::ROOT, &*window, Some(window.scale_factor() as f32), None, Some(8192)),
        ) else {
            el.exit();
            return;
        };
        let Ok(surface) = softbuffer::Surface::new(&context, window.clone()) else {
            el.exit();
            return;
        };
        if self.app.is_none() {
            self.app = Some(app::App::new(&self.ctx, self.settings.take().unwrap_or_default()));
            // Instance unique : un 2e lancement manuel ramène cette fenêtre (pas `--auto`, qui reste discret).
            if let Some(inst) = self.instance.take() {
                let (r, c) = (self.ramener.clone(), self.ctx.clone());
                inst.listen(move |args| {
                    if !args.iter().any(|a| a == "--auto") {
                        r.store(true, std::sync::atomic::Ordering::Relaxed);
                        c.request_repaint();
                    }
                });
            }
        }
        window.request_redraw();
        self.gfx = Some(Gfx { window, egui, _context: context, surface, renderer: EguiSoftwareRender::new(ColorFieldOrder::Bgra) });
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(gfx) = self.gfx.as_mut() else { return };
        let resp = gfx.egui.on_window_event(&gfx.window, &event);
        match event {
            WindowEvent::CloseRequested => {
                self.save();
                el.exit();
            }
            WindowEvent::RedrawRequested => self.paint(el),
            _ if resp.repaint => gfx.window.request_redraw(),
            _ => {}
        }
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, t: Instant) {
        if t <= Instant::now() {
            if let Some(g) = &self.gfx {
                g.window.request_redraw();
            }
        } else {
            self.next_repaint = Some(self.next_repaint.map_or(t, |n| n.min(t)));
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        match self.next_repaint {
            Some(t) if t <= Instant::now() => {
                self.next_repaint = None;
                if let Some(g) = &self.gfx {
                    g.window.request_redraw();
                }
                el.set_control_flow(ControlFlow::Wait);
            }
            Some(t) => el.set_control_flow(ControlFlow::WaitUntil(t)),
            None => el.set_control_flow(ControlFlow::Wait),
        }
    }

    fn exiting(&mut self, _el: &ActiveEventLoop) {
        self.save();
    }
}

fn capturer(window: &Window, f: &std::path::Path) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(h) = window.window_handle() else { return };
    let RawWindowHandle::Win32(w) = h.as_raw() else { return };
    if let Some((w, h, px)) = socle::testmode::capture(w.hwnd.get()) {
        let _ = image::save_buffer(f, &px, w, h, image::ColorType::Rgba8);
    }
}

static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn window_icon() -> Option<winit::window::Icon> {
    let img = image::load_from_memory(include_bytes!("../assets/icon64.png")).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    winit::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}

fn main() {
    T0.get_or_init(Instant::now);
    if socle::testmode::enabled() {
        socle::testmode::low_priority();
    }
    socle::crash::install(app::APP, env!("CARGO_PKG_VERSION"));
    socle::update::rollback_on_panic();
    if socle::update::startup() == socle::update::Startup::RolledBack {
        return;
    }
    socle::update::cleanup_old();
    let after_update = socle::update::is_after_update();
    let settings = app::Settings::load();

    // Tâche planifiée : photo du temps de jeu, puis fenêtre seulement si le bilan de la semaine écoulée est nouveau.
    let discret = std::env::args().any(|a| a == "--auto");
    if discret {
        sources::jeu::photographier(&sources::jeu::dossier_ultimate(), &app::photos_jeu(), chrono::Local::now().date_naive());
        if settings.dernier_vu.as_deref() == Some(app::derniere_semaine().to_string().as_str()) {
            return;
        }
    }
    // Déjà ouvert : l'instance existante suffit (on ne lui vole pas le focus).
    let mut instance = None;
    let args: Vec<String> = if discret { vec!["--auto".into()] } else { vec![] };
    match socle::instance::start(app::APP, &args) {
        socle::instance::Start::Forwarded if !after_update => return,
        socle::instance::Start::Primary(i) => instance = Some(i),
        _ => {}
    }

    let Ok(event_loop) = EventLoop::<Instant>::with_user_event().build() else { return };
    let proxy = event_loop.create_proxy();
    let ctx = egui::Context::default();
    ctx.set_request_repaint_callback(move |info| {
        let _ = proxy.send_event(Instant::now() + info.delay);
    });
    let mut runner = Runner { ctx, gfx: None, app: None, settings: Some(settings), next_repaint: None, discret, started: false, images: 0, instance, ramener: Default::default() };
    let _ = event_loop.run_app(&mut runner);
}
