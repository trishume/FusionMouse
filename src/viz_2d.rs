use glium_graphics::{
    Flip, Glium2d, GliumWindow, OpenGL, Texture, TextureSettings
};
use piston::input::*;
use piston::event_loop::EventLoop;
use piston::window::WindowSettings;
use graphics::draw_state::Blend;
use glutin::os::macos::WindowExt;
use glutin;

use objc;
use objc::runtime::{Class, Object, Sel, BOOL, YES, NO};
use objc::declare::ClassDecl;
use cocoa;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString, NSUInteger};
use cocoa::appkit::{self, NSApplication, NSColor, NSView, NSWindow, NSWindowStyleMask};

unsafe fn make_fullscreen_transparent_overlay(window: &glutin::GlWindow) {
    println!("Making window transparent");
    let os_window = window.get_nswindow() as id;
    os_window.setOpaque_(NO);
    os_window.setBackgroundColor_(NSColor::clearColor(nil));
}

pub fn run() {
    let opengl = OpenGL::V3_2;
    let (w, h) = (640, 480);
    let ref mut window: GliumWindow =
        WindowSettings::new("FusionMouse visualization", [w, h])
        .exit_on_esc(true).fullscreen(true).opengl(opengl).build().unwrap();


    let mut g2d = Glium2d::new(opengl, window);
    unsafe { make_fullscreen_transparent_overlay(&window.window.borrow().window) };
    while let Some(e) = window.next() {
        if let Some(args) = e.render_args() {
            use graphics::*;

            let mut target = window.draw();
            g2d.draw(&mut target, args.viewport(), |c, g| {
                clear([0.0; 4], g);
                // Rectangle::new([1.0, 0.0, 0.0, 1.0])
                //     .draw([0.0, 0.0, 100.0, 100.0], &c.draw_state, c.transform, g);

                // let draw_state = c.draw_state.blend(Blend::Alpha);
                // Rectangle::new([0.5, 1.0, 0.0, 0.3])
                //     .draw([50.0, 50.0, 100.0, 100.0], &draw_state, c.transform, g);

                // let transform = c.transform.trans(200.0, 200.0);
                // Ellipse::new_border([1.0, 0.0, 0.0, 1.0],1.0)
                //     .draw([0.0, 0.0, 50.0, 50.0], &c.draw_state, transform, g);
            });

            target.finish().unwrap();
        }
    }
}
