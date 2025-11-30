use crate::constant::{HEIGHT, SCROLL_STEP, WIDTH};
use crate::layout::block_layout::BlockLayoutRef;
use crate::layout::document_layout::{DocumentLayout, DocumentLayoutRef};
use crate::layout::draw_command::DrawCommand;
use crate::net::url::URL;
use crate::parser::html_node::HTMLNodeRef;
use crate::parser::html_parser::HTMLParser;
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
use skia_safe::{Color, ColorType, Paint, Surface, gpu};
use std::ffi::CString;
use std::num::NonZeroU32;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

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

#[derive(Default)]
pub struct Browser {
    env: Option<Env>,
    scroll: f32,
    nodes: Option<HTMLNodeRef>,
    document: Option<DocumentLayoutRef>,
    display_list: Vec<DrawCommand>,
}

impl Browser {
    pub fn new() -> Self {
        Self {
            scroll: 0.0,
            env: None,
            nodes: None,
            document: None,
            display_list: Vec::new(),
        }
    }

    pub fn load(&mut self, url: &URL) {
        // self.nodes = HTMLParser::new(url.request()).parse();
        self.nodes = HTMLParser::new(String::from("<html>
    <head></head>
    <body>
        <div class=\"sourceCode\" id=\"cb2\" data-replace=\"Layout/DocumentLayout\" data-dropline=\"display_list\"><pre class=\"sourceCode python\"><code class=\"sourceCode python\"><span id=\"cb2-1\"><a href=\"#cb2-1\" aria-hidden=\"true\" tabindex=\"-1\"></a><span class=\"kw\">class</span> Browser:</span>
<span id=\"cb2-2\"><a href=\"#cb2-2\" aria-hidden=\"true\" tabindex=\"-1\"></a>    <span class=\"kw\">def</span> load(<span class=\"va\">self</span>, url):</span>
<span id=\"cb2-3\"><a href=\"#cb2-3\" aria-hidden=\"true\" tabindex=\"-1\"></a>        <span class=\"co\"># ...</span></span>
<span id=\"cb2-4\"><a href=\"#cb2-4\" aria-hidden=\"true\" tabindex=\"-1\"></a>        <span class=\"va\">self</span>.document <span class=\"op\">=</span> Layout(<span class=\"va\">self</span>.nodes)</span>
<span id=\"cb2-5\"><a href=\"#cb2-5\" aria-hidden=\"true\" tabindex=\"-1\"></a>        <span class=\"va\">self</span>.document.layout()</span>
<span id=\"cb2-6\"><a href=\"#cb2-6\" aria-hidden=\"true\" tabindex=\"-1\"></a>        <span class=\"va\">self</span>.display_list <span class=\"op\">=</span> <span class=\"va\">self</span>.document.display_list</span>
<span id=\"cb2-7\"><a href=\"#cb2-7\" aria-hidden=\"true\" tabindex=\"-1\"></a>        <span class=\"co\">#...</span></span></code></pre></div>
    </body>
</html>")).parse();
        if let Some(node) = &self.nodes {
            // node.borrow().print_tree(0);

            let doc_rc = DocumentLayout::new(node.clone());
            self.document = Some(doc_rc.clone());

            doc_rc.borrow_mut().layout();
            doc_rc.borrow().print_tree(0);

            if let Some(block) = &doc_rc.borrow().child {
                self.display_list.clear();
                self.paint_tree(block.clone());
            }
        }

        for item in &self.display_list {
            println!("{}", item);
        }

        self.draw();
    }

    fn paint_tree(&mut self, block_rc: BlockLayoutRef) {
        let block = &*block_rc.borrow();
        self.display_list.append(&mut block.paint());

        for child in &block.children {
            self.paint_tree(child.clone());
        }
    }

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        event_loop.set_control_flow(ControlFlow::Wait);
        event_loop.run_app(self).expect("run() failed");
    }

    fn draw(&mut self) {
        if let Some(env) = &mut self.env {
            let canvas = env.surface.canvas();
            canvas.clear(Color::WHITE);

            canvas.save();

            let scale_factor = env.window.scale_factor() as f32;
            canvas.scale((scale_factor, scale_factor));

            let mut paint = Paint::default();
            paint.set_anti_alias(true);

            for cmd in self.display_list.iter() {
                if cmd.get_top() > self.scroll + HEIGHT {
                    continue;
                }
                if cmd.get_bottom() < self.scroll {
                    continue;
                }

                // reset paint's color
                paint.set_color(Color::BLACK);

                cmd.execute(self.scroll, canvas, &mut paint);
            }

            canvas.restore();

            env.gr_context.flush_and_submit();
            env.gl_surface.swap_buffers(&env.gl_context).unwrap();
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
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state.is_pressed() {
                    match key_event.logical_key {
                        Key::Named(NamedKey::ArrowDown) => {
                            self.scroll += SCROLL_STEP;

                            if let Some(env) = &self.env {
                                env.window.request_redraw();
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            self.scroll -= SCROLL_STEP;

                            if self.scroll < 0.0 {
                                self.scroll = 0.0;
                            }

                            if let Some(env) = &self.env {
                                env.window.request_redraw();
                            }
                        }
                        _ => (),
                    }
                }
            }

            _ => (),
        }
    }
}
