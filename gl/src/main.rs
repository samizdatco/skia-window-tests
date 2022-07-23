use std::time::{Duration, Instant};
use std::cell::RefCell;
use std::rc::{Rc};
use gl::{self, types::*};

mod contexts;
use contexts::ContextTracker;

use glutin::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
    GlProfile, PossiblyCurrent
};

use skia_safe::{
    gpu::{gl::FramebufferInfo, BackendRenderTarget, DirectContext, SurfaceOrigin},
    Color, ColorType, Surface, HSV, Color4f, Paint, Point, Rect, Size,
};

struct GLWindow {
    _id: usize,
    _ct: Rc<RefCell<ContextTracker>>,
    _surface: Option<Surface>,
    sk_context: DirectContext, // ← must be dropped before the WindowedContext!
    color: HSV
}

impl GLWindow {
    pub fn new(el:&EventLoop<()>, ct:&mut Rc<RefCell<ContextTracker>>) -> Self {
        let size:LogicalSize<i32> = LogicalSize::new(400, 300);

        let wb = WindowBuilder::new()
            .with_inner_size(size)
            .with_title("GL Window");
        let cb = glutin::ContextBuilder::new()
            .with_depth_buffer(0)
            .with_stencil_buffer(8)
            .with_pixel_format(24, 8)
            .with_gl_profile(GlProfile::Core);

        #[cfg(not(feature = "wayland"))]
        let cb = cb.with_double_buffer(Some(true));

        let windowed_context = cb.build_windowed(wb, &el).unwrap(); // ← this should be the safe bail-out point
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };
        gl::load_with(|s| windowed_context.get_proc_address(s));

        let sk_context = skia_safe::gpu::DirectContext::new_gl(None, None).unwrap();
        // let sf = windowed_context.window().scale_factor() as f32;
        // surface.canvas().scale((sf, sf));

        let _id = ct.borrow_mut().insert(windowed_context);

        GLWindow {
            _id,
            _ct: Rc::clone(&ct),
            _surface: None,
            sk_context,
            color: HSV::from((0.5, 1.0, 0.3))
        }
    }
    pub fn window_id<'a>(&'a self) -> glutin::window::WindowId {
        let ct = &mut self._ct.borrow_mut();
        let windowed_context = ct.get_current(self._id).unwrap();
        windowed_context.window().id()
    }

    pub fn with_gl_win<F>(&self, f:F)
        where F:Fn(&mut glutin::ContextWrapper<PossiblyCurrent, Window>)
    {
        let ct = &mut self._ct.borrow_mut();
        let windowed_context = ct.get_current(self._id).unwrap();
        f(windowed_context)
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>){
        self.with_gl_win(|win| win.resize(size));
        self._surface = None;
    }

    pub fn request_redraw(&mut self){
        self.with_gl_win(|win| win.window().request_redraw());
    }

    pub fn redraw(&mut self){
        self.color.h += 1.0;
        self.color.h %= 360.0;

        let s = 200.0 - 100.0 * ((self.color.h/180.0 * std::f32::consts::PI).cos() / 2.0 + 0.5);
        let mut x = (self.color.h/180.0 * std::f32::consts::PI).sin() / 2.0 + 0.5;
        let color:Color4f = self.color.to_color(255).into();

        if let Some(surface) = self.surface(){
            let canvas = surface.canvas();
            canvas.clear(Color::WHITE);

            let canvas_size = Size::from(canvas.base_layer_size());
            let rect_size = canvas_size * 0.9;
            let rect = Rect::from_point_and_size(
                Point::new(
                    (canvas_size.width - rect_size.width) / 2.0,
                    (canvas_size.height - rect_size.height) / 2.0,
                ),
                rect_size,
            );

            canvas.draw_rect(rect, &Paint::new(color, None));

            let h = canvas.base_layer_size().height as f32;
            let w = canvas.base_layer_size().width as f32;
            x *= w;
            let line = Rect::new(x-s/2.0, 0.0f32, x+s, h);

            let color:Color4f = Color::WHITE.into();
            canvas.draw_rect(line, &Paint::new(color, None));

        }

        self.sk_context.flush(None);
        self.with_gl_win(|win| win.swap_buffers().unwrap());
    }

    pub fn surface<'a>(&'a mut self) -> Option<&'a mut Surface> {
        if self._surface.is_none(){
            let ct = &mut self._ct.borrow_mut();
            let win = ct.get_current(self._id).unwrap();
            let pixel_format = win.get_pixel_format();
            let size = win.window().inner_size();
            let backend_render_target = BackendRenderTarget::new_gl(
                (
                    size.width.try_into().unwrap(),
                    size.height.try_into().unwrap(),
                ),
                pixel_format.multisampling.map(|s| s.try_into().unwrap()),
                pixel_format.stencil_bits.try_into().unwrap(),
                {
                    let mut fboid: GLint = 0;
                    unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };
                    FramebufferInfo {
                        fboid: fboid.try_into().unwrap(),
                        format: skia_safe::gpu::gl::Format::RGBA8.into(),
                    }
                },
            );
            self._surface = Some(Surface::from_backend_render_target(
                &mut self.sk_context,
                &backend_render_target,
                SurfaceOrigin::BottomLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .unwrap());
        }
        self._surface.as_mut()
    }
}

impl Drop for GLWindow {
    fn drop(&mut self) {
        let ct = &mut self._ct.borrow_mut();
        {
            ct.remove(self._id);
        }
        println!("Context with ID {:?} has been destroyed", self._id);
    }
}


fn main() {
    let el = EventLoop::new();
    let mut ct = Rc::new(RefCell::new(ContextTracker::default()));

    let mut windows = std::collections::HashMap::new();
    for index in 0..4 {
        let mut window = GLWindow::new(&el, &mut ct);
        window.color = match index {
            0 => HSV::from((0.0, 1.0, 0.2)),
            1 => HSV::from((90.0, 1.0, 0.5)),
            2 => HSV::from((180.0, 1.0, 0.75)),
            _ => HSV::from((270.0, 1.0, 1.0)),
        };

        let window_id = window.window_id();
        let ctx_id = window._id;
        windows.insert(window_id, window);
        println!("Created {:?} {}", window_id, ctx_id);
    }


    let frame_time = Duration::from_micros(1_000_000 / 60);
    let mut next_frame = Instant::now() + frame_time;

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        #[allow(deprecated)]
        match event {
            Event::LoopDestroyed => {}
            Event::WindowEvent { event, window_id, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    if let Some(window) = windows.get_mut(&window_id){
                        window.resize(physical_size);
                    }
                }
                WindowEvent::CloseRequested => {
                    if let Some(_) = windows.remove(&window_id) {
                        println!("Window with ID {:?} has been closed", window_id);
                    }
                    if windows.is_empty() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                // WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode,
                            modifiers,
                            ..
                        },
                    ..
                } => {
                    if modifiers.logo() {
                        if let Some(VirtualKeyCode::Q) = virtual_keycode {
                            *control_flow = ControlFlow::Exit;
                        }
                    }

                }
                _ => (),
            },
            Event::RedrawRequested(window_id) => {
                if let Some(window) = windows.get_mut(&window_id){
                    window.redraw();
                }
            }
            _ => {

                let now = Instant::now();
                if now >= next_frame{
                    while next_frame <= now {
                        next_frame += frame_time;
                    }

                    for (_, window) in windows.iter_mut() {
                        window.request_redraw();
                    }
                }


            },
        }
    });
}


