use std::sync::mpsc;

use cgmath::{self, Vector2};

use glium::{self, glutin, Surface};
use glium::index::PrimitiveType;
use glium::glutin::os::macos::WindowExt;

use objc::runtime::{YES, NO};
use cocoa::base::{id, nil};
use cocoa::appkit::{self, NSWindow, NSWindowStyleMask};

use signpost;

unsafe fn make_fullscreen_overlay(window: &glutin::GlWindow) {
    println!("Making window transparent");
    let os_window = window.get_nswindow() as id;
    os_window.setIgnoresMouseEvents_(YES);
    os_window.setHasShadow_(NO);
    os_window.setLevel_(1000);
    os_window.setStyleMask_(NSWindowStyleMask::NSBorderlessWindowMask);
    let main_frame = appkit::NSScreen::mainScreen(nil).frame();
    os_window.setFrame_display_(main_frame, YES);
}

#[derive(Copy,Clone)]
pub struct DebugPoint {
    pub offset: [f32; 2],
    pub color: [f32; 3],
}
implement_vertex!(DebugPoint, offset, color);

pub struct DebugFrame {
    pub points: Vec<DebugPoint>,
    pub display_width: f32,
    pub display_height: f32,
}

impl DebugFrame {
    pub fn add_point(&mut self, pt: Vector2<f32>, color: [f32; 3]) {
        self.points
            .push(DebugPoint {
                      offset: pt.into(),
                      color,
                  });
    }
}

pub struct DebugSender {
    tx: mpsc::Sender<DebugFrame>,
    proxy: glutin::EventsLoopProxy,
}

impl DebugSender {
    pub fn send(&self, frame: DebugFrame) {
        self.tx.send(frame).unwrap();
        self.proxy.wakeup().unwrap();
    }
}

pub struct DebugWindow {
    events_loop: glutin::EventsLoop,
    rx: mpsc::Receiver<DebugFrame>,
}

impl DebugWindow {
    pub fn new() -> (Self, DebugSender) {
        let (tx, rx) = mpsc::channel();
        let window = DebugWindow {
            events_loop: glutin::EventsLoop::new(),
            rx,
        };
        let sender = DebugSender {
            tx,
            proxy: window.events_loop.create_proxy(),
        };
        (window, sender)
    }

    pub fn run(self) {
        let DebugWindow {
            mut events_loop,
            rx,
        } = self;
        let window = glutin::WindowBuilder::new()
            .with_transparency(true)
            .with_decorations(false);
        let context = glutin::ContextBuilder::new().with_vsync(false);
        let display = glium::Display::new(window, context, &events_loop).unwrap();

        unsafe { make_fullscreen_overlay(&display.gl_window()) }

        // building the vertex buffer, which contains all the vertices that we will draw
        let vertex_buffer = {
            #[derive(Copy, Clone)]
            struct Vertex {
                position: [f32; 2],
            }

            implement_vertex!(Vertex, position);

            glium::VertexBuffer::new(&display,
                                     &[Vertex { position: [-0.5, -0.5] },
                                      Vertex { position: [0.0, 0.5] },
                                      Vertex { position: [0.5, -0.5] }])
                    .unwrap()
        };

        // building the index buffer
        let index_buffer =
            glium::IndexBuffer::new(&display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();

        // compiling shaders and linking them together
        let program = program!(&display,
            140 => {
                vertex: "
                    #version 140
                    uniform mat4 matrix;
                    in vec2 offset;
                    in vec2 position;
                    in vec3 color;
                    flat out vec3 vColor;
                    void main() {
                        gl_Position = matrix * vec4(position*10.0 + offset, 0.0, 1.0);
                        vColor = color;
                    }
                ",

                fragment: "
                    #version 140
                    flat in vec3 vColor;
                    out vec4 f_color;
                    void main() {
                        f_color = vec4(vColor, 1.0);
                    }
                "
            },
        )
                .unwrap();

        // Here we draw the black background and triangle to the screen using the previously
        // initialised resources.
        //
        // In this case we use a closure for simplicity, however keep in mind that most serious
        // applications should probably use a function that takes the resources as an argument.
        let draw = |frame: &DebugFrame| {
            signpost::start(4, &[0, 0, 0, signpost::Color::Purple as usize]);
            // building the uniforms
            let projection = cgmath::ortho(0.0,
                                           frame.display_width,
                                           frame.display_height,
                                           0.0,
                                           -1.0,
                                           1.0);
            let matrix: [[f32; 4]; 4] = projection.into();
            let uniforms = uniform! { matrix: matrix };

            let per_instance = glium::VertexBuffer::new(&display, &frame.points[..]).unwrap();

            // drawing a frame
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);
            target
                .draw((&vertex_buffer, per_instance.per_instance().unwrap()),
                      &index_buffer,
                      &program,
                      &uniforms,
                      &Default::default())
                .unwrap();
            target.finish().unwrap();
            signpost::end(4, &[0, 0, 0, signpost::Color::Purple as usize]);
        };

        // the main loop
        let mut running = true;
        while running {
            let handler = |event| match event {
                glutin::Event::WindowEvent { event, .. } => {
                    match event {
                        glutin::WindowEvent::Closed => running = false,
                        _ => (),
                    }
                }
                _ => (),
            };
            events_loop.poll_events(handler);

            let mut cur_frame = rx.recv().unwrap();
            while let Ok(frame) = rx.try_recv() {
                cur_frame = frame;
            }
            draw(&cur_frame);
        }
    }
}
