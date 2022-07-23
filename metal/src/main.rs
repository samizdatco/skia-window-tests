use std::collections::HashMap;
use std::cell::RefCell;
use std::time::{Duration, Instant};
use cocoa::{appkit::NSView, base::id as cocoa_id};
use core_graphics_types::geometry::CGSize;
use foreign_types_shared::{ForeignType, ForeignTypeRef};
use metal_rs::{Device, MTLPixelFormat, MetalLayer, CommandQueue};
use objc::{rc::autoreleasepool, runtime::YES};

use skia_safe::{
    scalar, HSV, Color4f, ColorType, Paint, Point, Rect, Size, Surface, Color,
    gpu::{mtl, BackendRenderTarget, DirectContext, SurfaceOrigin}
};

use winit::{
    dpi::{LogicalSize, LogicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::macos::WindowExtMacOS,
    window::{WindowBuilder, Window},
};

struct MetalWindow {
    window: Window,
    layer: MetalLayer,
    context: RefCell<DirectContext>,
    queue: CommandQueue,
    color: HSV
}

impl MetalWindow {
    pub fn new(window:Window) -> Self {
        let device = Device::system_default().expect("no device found");

        let layer = {
            let draw_size = window.inner_size();
            let layer = MetalLayer::new();
            layer.set_device(&device);
            layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
            layer.set_presents_with_transaction(false);

            unsafe {
                let view = window.ns_view() as cocoa_id;
                view.setWantsLayer(YES);
                view.setLayer(layer.as_ref() as *const _ as _);
            }
            layer.set_drawable_size(CGSize::new(draw_size.width as f64, draw_size.height as f64));
            layer
        };

        let queue = device.new_command_queue();

        let backend = unsafe {
            mtl::BackendContext::new(
                device.as_ptr() as mtl::Handle,
                queue.as_ptr() as mtl::Handle,
                std::ptr::null(),
            )
        };

        let context = RefCell::new(DirectContext::new_metal(&backend, None).unwrap());
        MetalWindow{ window, layer, context, queue, color: HSV::from((0.5, 1.0, 0.3)) }
    }

    pub fn resize(&self, size: PhysicalSize<u32>){
        self.layer.set_drawable_size(CGSize::new(size.width as f64, size.height as f64));
        self.window.request_redraw();
    }

    pub fn redraw(&mut self){
        self.color.h += 1.0;
        self.color.h %= 360.0;

        let s = 200.0 - 100.0 * ((self.color.h/180.0 * std::f32::consts::PI).cos() / 2.0 + 0.5);
        let mut x = (self.color.h/180.0 * std::f32::consts::PI).sin() / 2.0 + 0.5;
        let color:Color4f = self.color.to_color(255).into();

        if let Some(drawable) = self.layer.next_drawable() {
            let drawable_size = {
                let size = self.layer.drawable_size();
                Size::new(size.width as scalar, size.height as scalar)
            };

            let mut surface = unsafe {
                let texture_info =
                    mtl::TextureInfo::new(drawable.texture().as_ptr() as mtl::Handle);

                let backend_render_target = BackendRenderTarget::new_metal(
                    (drawable_size.width as i32, drawable_size.height as i32),
                    1,
                    &texture_info,
                );

                Surface::from_backend_render_target(
                    &mut self.context.borrow_mut(),
                    &backend_render_target,
                    SurfaceOrigin::TopLeft,
                    ColorType::BGRA8888,
                    None,
                    None,
                )
                .unwrap()
            };

            // draw(surface.canvas(), self.color.to_color(255).into());
            {
                let canvas = surface.canvas();
                let canvas_size = Size::from(canvas.base_layer_size());

                canvas.clear(Color4f::new(1.0, 1.0, 1.0, 1.0));

                let rect_size = canvas_size * 0.95;
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


            surface.flush_and_submit();
            drop(surface);

            let command_buffer = self.queue.new_command_buffer();
            command_buffer.present_drawable(drawable);
            command_buffer.commit();
        }

    }
}

fn main() {
    let size:LogicalSize<i32> = LogicalSize::new(400, 300);
    let mut loc:LogicalPosition<i32> = LogicalPosition::new(500, 300);

    let event_loop = EventLoop::new();

    let mut windows = HashMap::new();
    for win_id in 0..4 {
        let os_window = WindowBuilder::new()
          .with_inner_size(size)
          .with_position(loc)
          .with_title("Metal Window".to_string())
          .build(&event_loop)
          .unwrap();
        // println!("Opened a new window: {:?}", os_window.id());
        loc.x += 30;
        loc.y += 30;

        let mut window = MetalWindow::new(os_window);
        window.color = match win_id {
            0 => HSV::from((0.0, 1.0, 0.2)),
            1 => HSV::from((90.0, 1.0, 0.5)),
            2 => HSV::from((180.0, 1.0, 0.75)),
            _ => HSV::from((270.0, 1.0, 1.0)),
        };

        windows.insert(window.window.id(), window);
    }

    let frame_time = Duration::from_micros(1_000_000 / 60);
    let mut next_frame = Instant::now() + frame_time;

    event_loop.run(move |event, _, control_flow| {
        autoreleasepool(|| {
            *control_flow = ControlFlow::Poll;

            let now = Instant::now();
            if now > next_frame{
                while next_frame < now {
                    next_frame += frame_time;
                }
                for (_, win) in windows.iter_mut() {
                    win.redraw();
                }
            }

            match event {
                Event::WindowEvent { event, window_id } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            println!("Window {:?} has received the signal to close", window_id);

                            // This drops the window, causing it to close.
                            windows.remove(&window_id);

                            if windows.is_empty() {
                                control_flow.set_exit();
                            }
                        }
                        WindowEvent::Resized(size) => {
                            if let Some(window) = windows.get(&window_id){
                                window.resize(size);
                            }
                        }
                        _ => (),
                    }
                },
                Event::RedrawRequested(window_id) => {
                    if let Some(window) = windows.get_mut(&window_id){
                        window.redraw()
                    }
                }
                _ => {}
            }
        });
    });
}
