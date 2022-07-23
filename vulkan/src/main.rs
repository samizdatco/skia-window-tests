#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use env_logger::Env;

use skulpin_renderer::ash_window;
use skulpin_renderer::{ash, CoordinateSystem, CoordinateSystemHelper, RendererBuilder};
use ash::vk;

use skulpin_renderer::PhysicalSize as SRPhysicalSize;
use skulpin_renderer::LogicalSize as SRLogicalSize;
use skulpin_renderer::Window;
use std::ffi::CStr;
use ash::prelude::VkResult;
use std::os::raw::c_char;

#[derive(Clone)]
pub struct WinitWindow<'a> {
    window: &'a winit::window::Window,
}

impl<'a> WinitWindow<'a> {
    pub fn new(window: &'a winit::window::Window) -> Self {
        WinitWindow { window }
    }
}

impl<'a> Window for WinitWindow<'a> {
    fn physical_size(&self) -> SRPhysicalSize {
        let physical_size: winit::dpi::PhysicalSize<u32> = self.window.inner_size();
        SRPhysicalSize::new(physical_size.width, physical_size.height)
    }

    fn logical_size(&self) -> SRLogicalSize {
        let logical_size: winit::dpi::LogicalSize<u32> = self
            .window
            .inner_size()
            .to_logical(self.window.scale_factor());
        SRLogicalSize::new(logical_size.width, logical_size.height)
    }

    fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    unsafe fn create_vulkan_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> VkResult<vk::SurfaceKHR> {
        ash_window::create_surface(entry, instance, self.window, None)
    }

    fn extension_names(&self) -> VkResult<&'static [*const c_char]> {
        ash_window::enumerate_required_extensions(&self.window)
    }
}

use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::macos::WindowExtMacOS,
    window::{WindowBuilder},
};


fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let event_loop = EventLoop::new();

    let size:LogicalSize<i32> = LogicalSize::new(400, 300);
    let loc:LogicalPosition<i32> = LogicalPosition::new(500, 300);
    let logical_size = winit::dpi::LogicalSize::new(900.0, 600.0);
    let visible_range = skia_safe::Rect {
        left: 0.0,
        right: logical_size.width as f32,
        top: 0.0,
        bottom: logical_size.height as f32,
    };
    let scale_to_fit = skia_safe::matrix::ScaleToFit::Center;


    let winit_window = WindowBuilder::new()
        .with_inner_size(size)
        .with_position(loc)
        .with_title("Vulkan Window".to_string())
        .build(&event_loop)
        .unwrap();

    let window = WinitWindow::new(&winit_window);

    let renderer = RendererBuilder::new()
        .use_vulkan_debug_layer(true)
        .coordinate_system(CoordinateSystem::VisibleRange(
            visible_range,
            scale_to_fit,
        ))
        .build(&window);

    if let Err(e) = renderer {
        println!("Error during renderer construction: {:?}", e);
        return;
    }

    let mut renderer = renderer.unwrap();

    // Increment a frame count so we can render something that moves
    let mut frame_count = 0;

    // Start the window event loop. Winit will not return once run is called. We will get notified
    // when important events happen.
    event_loop.run(move |event, _window_target, control_flow| {
        let window = WinitWindow::new(&winit_window);

        match event {
            //
            // Halt if the user requests to close the window
            //
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,

            //
            // Close if the escape key is hit
            //
            winit::event::Event::WindowEvent {
                event:
                    winit::event::WindowEvent::KeyboardInput {
                        input:
                            winit::event::KeyboardInput {
                                virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => *control_flow = winit::event_loop::ControlFlow::Exit,

            //
            // Request a redraw any time we finish processing events
            //
            winit::event::Event::MainEventsCleared => {
                // Queue a RedrawRequested event.
                winit_window.request_redraw();
            }

            //
            // Redraw
            //
            winit::event::Event::RedrawRequested(_window_id) => {
                if let Err(e) = renderer.draw(&window, |canvas, coordinate_system_helper| {
                    draw(canvas, coordinate_system_helper, frame_count);
                    frame_count += 1;
                }) {
                    println!("Error during draw: {:?}", e);
                    *control_flow = winit::event_loop::ControlFlow::Exit
                }
            }

            //
            // Ignore all other events
            //
            _ => {}
        }
    });
}

/// Called when winit passes us a WindowEvent::RedrawRequested
fn draw(
    canvas: &mut skia_safe::Canvas,
    _coordinate_system_helper: CoordinateSystemHelper,
    frame_count: i32,
) {
    // Generally would want to clear data every time we draw
    canvas.clear(skia_safe::Color::from_argb(0, 0, 0, 255));

    // Floating point value constantly moving between 0..1 to generate some movement
    let f = ((frame_count as f32 / 30.0).sin() + 1.0) / 2.0;

    // Make a color to draw with
    let mut paint = skia_safe::Paint::new(skia_safe::Color4f::new(1.0 - f, 0.0, f, 1.0), None);
    paint.set_anti_alias(true);
    paint.set_style(skia_safe::paint::Style::Stroke);
    paint.set_stroke_width(2.0);

    // Draw a line
    canvas.draw_line(
        skia_safe::Point::new(100.0, 500.0),
        skia_safe::Point::new(800.0, 500.0),
        &paint,
    );

    // Draw a circle
    canvas.draw_circle(
        skia_safe::Point::new(200.0 + (f * 500.0), 420.0),
        50.0,
        &paint,
    );

    // Draw a rectangle
    canvas.draw_rect(
        skia_safe::Rect {
            left: 10.0,
            top: 10.0,
            right: 890.0,
            bottom: 590.0,
        },
        &paint,
    );

    let mut font = skia_safe::Font::default();
    font.set_size(100.0);

    canvas.draw_str("Hello World", (65, 200), &font, &paint);
    canvas.draw_str("Holo Whirled", (68, 203), &font, &paint);
    canvas.draw_str("Hollow Whorled", (71, 206), &font, &paint);
}
