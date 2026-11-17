//! Loop aplikasi: winit + wgpu + egui (SDD Bagian 4-5).
//!
//! Pola integrasi mengikuti contoh resmi `egui-wgpu` (winit 0.30):
//! - `android_main` menerima `AndroidApp`, lalu `EventLoop` dibuat dengan
//!   `with_android_app` (fitur `android-native-activity` winit).
//! - Window dibuat saat event `Resumed`; surface wgpu dibuat ulang tiap
//!   resume dan dibuang saat `Suspended` (di Android, surface tidak
//!   boleh dipakai saat activity dijeda).
//! - egui digambar lewat `egui-winit::State` + `egui-wgpu::Renderer`
//!   yang dikelola manual (bukan `eframe`), supaya kontrol penuh atas
//!   format surface dan repaint.
//! - Repaint dijadwalkan dari `ViewportOutput::repaint_delay`: loop
//!   berjalan terus hanya bila egui minta repaint segera, selebihnya
//!   `WaitUntil` (hemat baterai saat layar diam).

use std::sync::Arc;
use std::time::Instant;

use egui::{Context, ViewportId};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

use crate::app::App;

/// Event internal untuk membangunkan loop (mis. egui minta repaint).
/// (Hanya dipakai di Android; di host muncul warning dead-code.)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum AppEvent {
    Repaint,
}

/// Warna latar (abyss v2, `#0d0a1a`) untuk clear color tiap frame.
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0x0d as f64 / 255.0,
    g: 0x0a as f64 / 255.0,
    b: 0x1a as f64 / 255.0,
    a: 1.0,
};

/// Titik masuk Android: dipanggil dari `android_main` di `lib.rs`.
#[cfg(target_os = "android")]
pub fn run_android(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .with_android_app(app)
        .build()
        .expect("gagal membuat event loop winit");
    let mut engine = Engine::new(&event_loop);
    let _ = event_loop.run_app(&mut engine);
}

/// State loop + grafis. Dibuat sekali per proses; window/surface
/// dibuat ulang tiap resume (Android) atau sekali (host).
/// (Hanya dipakai di Android; di host muncul warning dead-code.)
#[allow(dead_code)]
pub struct Engine {
    egui_ctx: Context,
    egui_state: Option<egui_winit::State>,
    window: Option<Arc<Window>>,
    instance: Option<wgpu::Instance>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    surface_format: wgpu::TextureFormat,
    renderer: Option<egui_wgpu::Renderer>,
    app: App,
    proxy: EventLoopProxy<AppEvent>,
    /// Kapan repaint berikutnya jatuh tempo (`None` = segera).
    next_repaint: Option<Instant>,
    destroyed: bool,
}

#[allow(dead_code)] // `new` hanya dipakai dari `run_android` (cfg Android).
impl Engine {
    fn new(event_loop: &EventLoop<AppEvent>) -> Self {
        let proxy = event_loop.create_proxy();
        let egui_ctx = Context::default();
        crate::ui::theme::install_theme(&egui_ctx);
        // Minta repaint lewat proxy begitu egui menjadwalkannya
        // (animasi, timer, dsb.) — loop dibangunkan via UserEvent.
        let wake_proxy = proxy.clone();
        egui_ctx.set_request_repaint_callback(move |_| {
            let _ = wake_proxy.send_event(AppEvent::Repaint);
        });
        Self {
            egui_ctx,
            egui_state: None,
            window: None,
            instance: None,
            device: None,
            queue: None,
            surface: None,
            surface_format: wgpu::TextureFormat::Bgra8Unorm,
            renderer: None,
            app: App::new(),
            proxy,
            next_repaint: None,
            destroyed: false,
        }
    }

    /// Buat window + surface + device + renderer bila belum ada.
    /// Dipanggil dari `resumed` dan dari `RedrawRequested` bila
    /// surface hilang (mis. setelah `Suspended`).
    fn ensure_graphics(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = event_loop
                .create_window(WindowAttributes::default().with_title("Dungeon Learning"))
                .expect("gagal membuat window winit");
            self.window = Some(Arc::new(window));
        }
        let window = self.window.clone().expect("window baru saja dibuat");

        if self.instance.is_none() {
            let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
            desc.backends = wgpu::Backends::all();
            self.instance = Some(wgpu::Instance::new(desc));
        }
        let instance = self.instance.as_ref().expect("instance baru saja dibuat");

        if self.surface.is_none() {
            match instance.create_surface(Arc::clone(&window)) {
                Ok(surface) => self.surface = Some(surface),
                Err(_) => return, // belum resumed penuh; coba lagi di frame berikut
            }
            // egui-state dibuat ulang bersama window (clipboard/IME).
            self.egui_state = Some(egui_winit::State::new(
                self.egui_ctx.clone(),
                ViewportId::ROOT,
                &window,
                None,
                None,
                None,
            ));
        }
        let surface = self.surface.as_ref().expect("surface baru saja dibuat");

        if self.device.is_none() {
            let adapter = match pollster::block_on(instance.request_adapter(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(surface),
                    force_fallback_adapter: false,
                    apply_limit_buckets: false,
                },
            )) {
                Ok(a) => a,
                Err(_) => return, // belum ada adapter; coba lagi nanti
            };
            let caps = surface.get_capabilities(&adapter);
            self.surface_format = caps
                .formats
                .iter()
                .copied()
                .find(|f| {
                    matches!(
                        f,
                        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                    )
                })
                .unwrap_or(caps.formats[0]);
            let (device, queue) = match pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("dungeon-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    ..Default::default()
                },
            )) {
                Ok(pair) => pair,
                Err(_) => return,
            };
            self.renderer = Some(egui_wgpu::Renderer::new(
                &device,
                self.surface_format,
                egui_wgpu::RendererOptions::default(),
            ));
            self.device = Some(device);
            self.queue = Some(queue);
        }

        self.configure_surface();
    }

    fn configure_surface(&mut self) {
        let (Some(window), Some(surface), Some(device)) =
            (self.window.as_ref(), self.surface.as_ref(), self.device.as_ref())
        else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        surface.configure(
            device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.surface_format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }

    /// Satu frame egui + wgpu. Dipanggil dari `RedrawRequested`.
    fn paint(&mut self, event_loop: &ActiveEventLoop) {
        self.ensure_graphics(event_loop);
        let (Some(window), Some(device), Some(queue), Some(surface), Some(renderer)) = (
            self.window.clone(),
            self.device.as_ref(),
            self.queue.as_ref(),
            self.surface.as_ref(),
            self.renderer.as_mut(),
        ) else {
            return;
        };
        let Some(egui_state) = self.egui_state.as_mut() else {
            return;
        };

        let raw_input = egui_state.take_egui_input(&window);
        let full_output = self.egui_ctx.run_ui(raw_input, |ui| {
            self.app.show(ui);
        });
        egui_state.handle_platform_output(&window, full_output.platform_output);

        // Unggah tekstur egui yang berubah.
        for (id, deltas) in &full_output.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(device, queue, *id, delta);
            }
        }
        for id in &full_output.textures_delta.free {
            renderer.free_texture(id);
        }

        let pixels_per_point = full_output.pixels_per_point;
        let clipped = self
            .egui_ctx
            .tessellate(full_output.shapes, pixels_per_point);
        let size = window.inner_size();
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point,
        };

        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface = None;
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => return,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("egui") });
        let extra_buffers =
            renderer.update_buffers(device, queue, &mut encoder, &clipped, &screen_descriptor);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // `forget_lifetime` (wgpu 30): egui-wgpu 0.37 meminta
            // `RenderPass<'static>`; pass dipakai sebelum encoder selesai.
            renderer.render(&mut pass.forget_lifetime(), &clipped, &screen_descriptor);
        }
        queue.submit(
            std::iter::once(encoder.finish()).chain(extra_buffers.into_iter()),
        );
        queue.present(frame);

        // Jadwalkan repaint berikutnya dari egui.
        let delay = full_output.viewport_output[&ViewportId::ROOT].repaint_delay;
        if delay.is_zero() {
            self.next_repaint = None;
            event_loop.set_control_flow(ControlFlow::Poll);
        } else {
            let at = Instant::now() + delay;
            self.next_repaint = Some(at);
            event_loop.set_control_flow(ControlFlow::WaitUntil(at));
        }
    }
}

impl ApplicationHandler<AppEvent> for Engine {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.ensure_graphics(event_loop);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        // Android: buang window + surface saat dijeda; device dan
        // renderer dipertahankan, dibuat ulang saat resume.
        self.surface = None;
        self.window = None;
        self.egui_state = None;
        self.next_repaint = None;
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.clone() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => {
                self.destroyed = true;
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                self.configure_surface();
            }
            WindowEvent::RedrawRequested => {
                self.paint(event_loop);
            }
            _ => {}
        }
        // Teruskan semua event ke egui (sentuh, keyboard, IME, dsb.).
        if let Some(state) = self.egui_state.as_mut() {
            let _ = state.on_window_event(&window, &event);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: AppEvent) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.destroyed || self.window.is_none() {
            return;
        }
        match self.next_repaint {
            None => {
                // egui minta repaint segera (animasi/loop) — gambar terus.
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                event_loop.set_control_flow(ControlFlow::Poll);
            }
            Some(at) => {
                if at <= Instant::now() {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    event_loop.set_control_flow(ControlFlow::Poll);
                } else {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(at));
                }
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        // Bangun setelah resume: pastikan grafis siap.
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}
