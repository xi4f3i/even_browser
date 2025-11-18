use std::ffi::CString;
use std::num::NonZeroU32;

use gl_rs as gl;
use gl_rs::types::GLint;
use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::NotCurrentGlContext;
use glutin::surface::{
    GlSurface, Surface as GlutinSurface, SurfaceAttributesBuilder, WindowSurface,
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use skia_safe::gpu::gl::Format;
use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::gpu::gl::Interface;
use skia_safe::gpu::{DirectContext, SurfaceOrigin, backend_render_targets};
use skia_safe::{Color, ColorType, Font, FontMgr, Paint, Point, Surface, gpu};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::constant::{HEIGHT, VSTEP, WIDTH};
use crate::layout::{Glyph, layout};
use crate::lexer::lex;
use crate::url::URL;

// Guarantee the drop order inside the FnMut closure. `Window` _must_ be dropped after
// `DirectContext`.
//
// <https://github.com/rust-skia/rust-skia/issues/476>
#[derive(Debug)]
struct Env {
    surface: Surface,
    gl_surface: GlutinSurface<WindowSurface>,
    gr_context: DirectContext,
    gl_context: PossiblyCurrentContext,
    window: Window,
}

#[derive(Default, Debug)]
pub struct Browser {
    env: Option<Env>,
    display_list: Vec<Glyph>,
    scroll: f32,
}

impl Browser {
    pub fn init(&mut self) {
        self.scroll = 0.0;

        let event_loop = EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(self).expect("run() failed");
    }

    pub fn load(&mut self, url: &URL) {
        let body = url.request();
        let text = lex(&body);
        self.display_list = layout(&text);
        // self.draw();
    }

    fn draw(&mut self) {
        if let Some(env) = &mut self.env {
            let canvas = env.surface.canvas();
            canvas.clear(Color::WHITE);

            canvas.save();

            let scale_factor = env.window.scale_factor() as f32;
            canvas.scale((scale_factor, scale_factor));

            let font_mgr = FontMgr::new();
            let typeface = font_mgr
                .match_family_style("PingFang SC", Default::default())
                .expect("Cannot find PingFang SC font");
            let font = Font::new(typeface, VSTEP);

            let mut paint = Paint::default();
            paint.set_color(Color::BLACK);
            paint.set_anti_alias(true);

            let mut buf = [0u8; 4];

            for glyph in &self.display_list {
                if glyph.y > self.scroll + HEIGHT {
                    continue;
                }

                if glyph.y + VSTEP < self.scroll {
                    continue;
                }

                let centered_point = Point::new(glyph.x, glyph.y);

                canvas.draw_str(glyph.c.encode_utf8(&mut buf), centered_point, &font, &paint);
            }

            canvas.restore();

            env.gr_context.flush_and_submit();
            env.gl_surface.swap_buffers(&env.gl_context).unwrap();

            // Queue a RedrawRequested event.
            //
            // You only need to call this if you've determined that you need to redraw in
            // applications which do not always need to. Applications that redraw continuously
            // can render here instead.
            env.window.request_redraw();
        }
    }
}

impl ApplicationHandler for Browser {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("ApplicationHandler::resumed");
        let window_attributes = WindowAttributes::default()
            .with_title("Even Browser")
            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true);

        let display_builder =
            DisplayBuilder::new().with_window_attributes(window_attributes.into());

        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
                // Find the config with the minimum number of samples. Usually Skia takes care of
                // anti-aliasing and may not be able to create appropriate Surfaces for samples > 0.
                // See https://github.com/rust-skia/rust-skia/issues/782
                // And https://github.com/rust-skia/rust-skia/issues/764
                configs
                    .reduce(|accum, config| {
                        let transparency_check = config.supports_transparency().unwrap_or(false)
                            & !accum.supports_transparency().unwrap_or(false);

                        if transparency_check || config.num_samples() < accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        println!("Picked a config with {} samples", gl_config.num_samples());
        let window = window.expect("Could not create window with OpenGL context");
        let window_handle = window
            .window_handle()
            .expect("Failed to retrieve RawWindowHandle");
        let raw_window_handle = window_handle.as_raw();

        // The context creation part. It can be created before surface and that's how
        // it's expected in multithreaded + multiwindow operation mode, since you
        // can send NotCurrentContext, but not Surface.
        let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));

        // Since glutin by default tries to create OpenGL core context, which may not be
        // present we should try gles.
        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(Some(raw_window_handle));
        let not_current_gl_context = unsafe {
            gl_config
                .display()
                .create_context(&gl_config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_config
                        .display()
                        .create_context(&gl_config, &fallback_context_attributes)
                        .expect("failed to create context")
                })
        };

        let (width, height): (u32, u32) = window.inner_size().into();

        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );

        let gl_surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .expect("Could not create gl window surface")
        };

        let gl_context = not_current_gl_context
            .make_current(&gl_surface)
            .expect("Could not make GL context current when setting up skia renderer");

        gl::load_with(|s| {
            gl_config
                .display()
                .get_proc_address(CString::new(s).unwrap().as_c_str())
        });

        let interface = Interface::new_load_with(|name| {
            if name == "eglGetCurrentDisplay" {
                return std::ptr::null();
            }
            gl_config
                .display()
                .get_proc_address(CString::new(name).unwrap().as_c_str())
        })
        .expect("Could not create interface");

        let mut gr_context = gpu::direct_contexts::make_gl(interface, None)
            .expect("Could not create direct context");

        let fb_info = {
            let mut fboid: GLint = 0;
            unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

            FramebufferInfo {
                fboid: fboid.try_into().unwrap(),
                format: Format::RGBA8.into(),
                ..Default::default()
            }
        };

        let num_samples = gl_config.num_samples() as usize;
        let stencil_size = gl_config.stencil_size() as usize;

        let size = window.inner_size();
        let size = (
            size.width.try_into().expect("Could not convert width"),
            size.height.try_into().expect("Could not convert height"),
        );
        let backend_render_target =
            backend_render_targets::make_gl(size, num_samples, stencil_size, fb_info);

        let surface = gpu::surfaces::wrap_backend_render_target(
            &mut gr_context,
            &backend_render_target,
            SurfaceOrigin::BottomLeft,
            ColorType::RGBA8888,
            None,
            None,
        )
        .expect("Could not create skia surface");

        self.env = Some(Env {
            window,
            surface,
            gl_context,
            gr_context,
            gl_surface,
        });

        self.draw();

        if let Some(env) = &self.env {
            env.window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            // TODO: handle resize event
            // WindowEvent::Resized => {
            //     println!("WindowEvent::Resized");
            // }
            WindowEvent::RedrawRequested => {
                // println!("WindowEvent::RedrawRequested");
                // self.draw();
            }
            _ => (),
        }
    }
}
