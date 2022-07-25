#![allow(dead_code)]
#![allow(unused_imports)]

use log;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::{sync::mpsc, thread};
use skulpin::{CoordinateSystemHelper, CoordinateSystem, Renderer, RendererBuilder};
use skulpin::rafx::api::RafxExtents2D;
use skia_safe::{Point, Size, Rect, Color, Color4f, HSV, Paint};
use winit::{
    dpi::{LogicalSize, LogicalPosition, PhysicalSize},
    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState},
    event_loop::{ControlFlow, EventLoop},
    platform::macos::WindowExtMacOS,
    window::{WindowBuilder, Window},
};

struct VulkanWindow{
    renderer: Arc<Mutex<Renderer>>,
    window: Window,
    color: HSV
}

unsafe impl Send for VulkanWindow {}

impl VulkanWindow {
    pub fn new(window:Window) -> Option<Self> {
        let window_size = window.inner_size();
        let window_extents = RafxExtents2D {
            width: window_size.width,
            height: window_size.height,
        };

        let renderer = RendererBuilder::new()
            .coordinate_system(CoordinateSystem::Logical)
            .build(&window, window_extents);

        let color = HSV::from((0.5, 1.0, 0.3));

        renderer.map(|renderer|
            Self{ window, renderer:Arc::new(Mutex::new(renderer)), color }
        ).ok()
    }

    pub fn resize(&mut self, _size: PhysicalSize<u32>) {
        self.redraw().ok();
    }

    pub fn redraw(&mut self) -> Result<(), String>{
        self.color.h += 1.0;
        self.color.h %= 360.0;

        let window_size = self.window.inner_size();
        let window_extents = RafxExtents2D {
            width: window_size.width,
            height: window_size.height,
        };

        let mut s = 1.0/3.0 - 1.0/4.0 * ((self.color.h/180.0 * std::f32::consts::PI).cos() / 2.0 + 0.5);
        let mut x = (self.color.h/180.0 * std::f32::consts::PI).sin() / 2.0 + 0.5;
        let color:Color4f = self.color.to_color(255).into();

        if let Err(e) = self.renderer.lock().unwrap().draw(
            window_extents,
            self.window.scale_factor(),
            |canvas, coords| {
                let cw = coords.window_logical_size().width as f32;
                let ch = coords.window_logical_size().height as f32;
                let w = 0.95 * cw;
                let h = 0.95 * ch;

                canvas.clear(Color4f::new(1.0, 1.0, 1.0, 1.0));

                let rect = Rect::from_point_and_size(
                    Point::new(
                        (cw - w) / 2.0,
                        (ch - h) / 2.0,
                    ),
                    (w,  h),
                );
                canvas.draw_rect(rect, &Paint::new(color, None));

                x *= w;
                s *= w;
                let line = Rect::new(x-s/2.0, 0.0f32, x+s, ch);

                let color:Color4f = Color::WHITE.into();
                canvas.draw_rect(line, &Paint::new(color, None));
            },
        ){
            Err(format!("Error in draw routine {}", e))
        }else{
            Ok(())
        }
    }
}


fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let event_loop = EventLoop::new();

    const WINDOW_COUNT: usize = 4;
    let size:LogicalSize<i32> = LogicalSize::new(400, 300);
    let mut loc:LogicalPosition<i32> = LogicalPosition::new(500, 300);

    let mut window_senders = HashMap::with_capacity(WINDOW_COUNT);
    for win_id in 0..WINDOW_COUNT {

        let os_window = WindowBuilder::new()
            .with_inner_size(size)
            .with_position(loc)
            .with_title("Vulkan Window".to_string())
            .build(&event_loop)
            .unwrap();

        loc.x += 30;
        loc.y += 30;
        let mut video_modes: Vec<_> = os_window.current_monitor().unwrap().video_modes().collect();
        let mut video_mode_id = 0usize;

        let (tx, rx) = mpsc::channel();
        window_senders.insert(os_window.id(), tx);

        let mut window = VulkanWindow::new(os_window).unwrap();
        window.color = match win_id {
            0 => HSV::from((0.0, 1.0, 0.2)),
            1 => HSV::from((90.0, 1.0, 0.5)),
            2 => HSV::from((180.0, 1.0, 0.75)),
            _ => HSV::from((270.0, 1.0, 1.0)),
        };

        thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                match event {

                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Moved { .. } => {
                            // We need to update our chosen video mode if the window
                            // was moved to an another monitor, so that the window
                            // appears on this monitor instead when we go fullscreen
                            let previous_video_mode = video_modes.get(video_mode_id).cloned();
                            video_modes = window.window.current_monitor().unwrap().video_modes().collect();
                            video_mode_id = video_mode_id.min(video_modes.len());
                            let video_mode = video_modes.get(video_mode_id);

                            // Different monitors may support different video modes,
                            // and the index we chose previously may now point to a
                            // completely different video mode, so notify the user
                            if video_mode != previous_video_mode.as_ref() {
                                println!(
                                    "Window moved to another monitor, picked video mode: {}",
                                    video_modes.get(video_mode_id).unwrap()
                                );
                            }
                        },
                        WindowEvent::Resized(size) => {
                            window.resize(size);

                        },
                        _ => {}
                    }
                    Event::RedrawRequested(_) => {
                        window.redraw().unwrap();
                    },
                    _ => {}
                }
            }
        });
    }

    let frame_time = Duration::from_micros(1_000_000 / 60);
    let mut next_frame = Instant::now() + frame_time;

    // Start the window event loop. Winit will not return once run is called. We will get notified
    // when important events happen.
    event_loop.run(move |event, _window_target, control_flow| {
        match event {
            //
            // Halt if the user requests to close the window
            //
            Event::WindowEvent { event:ref win_event, window_id } => match win_event {

                WindowEvent::KeyboardInput { input: KeyboardInput { state: ElementState::Released, virtual_keycode: Some(VirtualKeyCode::Escape), .. }, .. } |
                WindowEvent::CloseRequested |
                WindowEvent::Destroyed => {
                    window_senders.remove(&window_id);
                    if window_senders.is_empty(){
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {
                    if let Some(tx) = window_senders.get(&window_id) {
                        if let Some(event) = event.to_static() {
                            tx.send(event).unwrap();
                        }
                    }
                }
            },
            Event::RedrawRequested(window_id) => {
                if let Some(tx) = window_senders.get(&window_id) {
                    if let Some(event) = event.to_static() {
                        tx.send(event).unwrap();
                    }
                }
            }

            Event::MainEventsCleared => {
                // Queue a RedrawRequested event.
                let now = Instant::now();
                if now > next_frame{
                    while next_frame < now {
                        next_frame += frame_time;
                    }
                    for (win_id, tx) in window_senders.iter() {
                        if let Some(event) = Event::RedrawRequested(*win_id).to_static() {
                            tx.send(event).unwrap();
                        }
                    }
                }
            }
            _ => {}
        }
    });
}
