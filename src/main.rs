extern crate linuxtrack_sys;
#[cfg(feature = "tobii")]
extern crate tobii_sys;
extern crate cgmath;
extern crate enigo;
extern crate signpost;

#[cfg(feature = "viz-2d")]
#[macro_use]
extern crate glium;
#[cfg(feature = "viz-2d")]
extern crate cocoa;
#[cfg(feature = "viz-2d")]
extern crate objc;

mod inputs;
mod ltr_input;
#[cfg(feature = "tobii")]
mod tobii_input;
mod transforms;

#[cfg(feature = "viz-2d")]
mod viz_2d;
#[cfg(feature = "viz-2d")]
use viz_2d::{DebugSender, DebugWindow, DebugFrame, DebugPoint};
#[cfg(not(feature = "viz-2d"))]
struct DebugSender();

use cgmath::{vec2, Vector2};
use enigo::{Enigo, MouseControllable};

use std::sync::mpsc::Receiver;
use std::time::Instant;
use std::mem;
use std::cmp::{min, max};
use std::thread;

use inputs::{InputPool, Input};
use transforms::*;

fn calc_dt(tick: Instant, last_tick: &mut Instant) -> f32 {
    let dur = tick.duration_since(*last_tick);
    let dt = dur.as_secs() as f32 + dur.subsec_nanos() as f32 * 1.0e-9;
    mem::replace(last_tick, tick);
    dt
}

fn run_pipeline(rx: Receiver<Input>, debug: DebugSender) {
    // configuration
    let accel = Acceleration {
        cd_min: 8.0, // min gain
        cd_max: 65.0, // max gain
        v_min: 0.0004, // input velocity lower bound
        v_max: 0.0025, // input velocity upper bound
        lambda: 1000.0, // slope of curve at inflection point
        ratio: 0.7, // where inflection lies between v_min and v_max
    };
    let polymouse_params = PolyMouseParams {
        min_jump: 100.0,
        speed_expand_factor: 0.0, // TODO translate delta->speed
        head_smoothing_factor: 1.0, // TODO tune for dt
        throw_thresh_speed: 300.0, // pixels per second
        throw_speed: 8000.0, // pixels per second
        small_jump_factor: 0.75,
    };

    // input state
    let mut raw_head_pose: Vector2<f32> = vec2(0.0, 0.0);
    let mut raw_gaze: Vector2<f32> = vec2(0.0, 0.0);

    // pipeline state
    let mut last_head_tick = Instant::now();
    let mut last_gaze_tick = Instant::now();
    let mut head_filter = VecOneEuroFilter::new(6.0, 1000.0, 1.0);
    let mut last_head_pose: Option<Vector2<f32>> = None;

    let mut poly_mouse = PolyMouseTransform::new(polymouse_params.clone());

    let mut fixation_filter = FixationFilter::new(0.03, 150.0);
    let mut gaze_pt: Vector2<f32> = vec2(0.0, 0.0);
    let mut px_gaze: Vector2<f32> = vec2(0.0, 0.0);

    let mut enigo = Enigo::new();

    loop {
        // update input state =========================
        let mut tick_gaze = false;
        let mut tick_head = false;
        match rx.recv().unwrap() {
            Input::LinuxTrackHead { yaw, pitch } => {
                raw_head_pose = vec2(yaw, pitch) * -1.0;
                tick_head = true;
            }
            #[cfg(feature = "tobii")]
            Input::TobiiGaze { x, y } => {
                raw_gaze = vec2(x, y);
                tick_gaze = true;
            }
            Input::Shutdown => break,
        }
        let _signpost = signpost::AutoTrace::new(1, &[0, 0, 0, signpost::Color::Blue as usize]);

        let tick = Instant::now();
        let (display_width, display_height) = Enigo::main_display_size();

        // compute pipeline results ===================
        if tick_head {
            let dt = calc_dt(tick, &mut last_head_tick);
            let smoothed_head = head_filter.filter(raw_head_pose, dt);
            // let smoothed_head = raw_head_pose;

            let head_delta = match last_head_pose {
                Some(last_pose) => smoothed_head - last_pose,
                None => vec2(0.0, 0.0),
            };
            last_head_pose = Some(smoothed_head);

            let head_cursor_move = vec2(accel.transform(head_delta.x, dt),
                                        accel.transform(head_delta.y, dt));

            let (mouse_x, mouse_y) = Enigo::mouse_location();
            let mouse_pt = vec2(mouse_x, mouse_y);
            let dest = poly_mouse.transform(gaze_pt, mouse_pt, head_cursor_move, dt);
            let confined = vec2(max(0, min(display_width as i32, dest.x)),
                                max(0, min(display_height as i32, dest.y)));

            if confined != mouse_pt {
                enigo.mouse_move_to(confined.x, confined.y);
            }

            // debugging =====================
            #[cfg(feature = "viz-2d")]
            {
                let mut debug_frame = DebugFrame {
                    points: Vec::with_capacity(4),
                    display_width: display_width as f32,
                    display_height: display_height as f32,
                };
                let circle = DebugPoint {
                    offset: [dest.x as f32, dest.y as f32],
                    color: [0.0, 1.0, 0.0],
                    size: polymouse_params.min_jump*2.0,
                };
                debug_frame.points.push(circle);
                let circle2 = DebugPoint {
                    offset: poly_mouse.last_jump_destination.into(),
                    color: [0.0, 1.0, 0.0],
                    size: polymouse_params.min_jump*polymouse_params.small_jump_factor*2.0,
                };
                debug_frame.points.push(circle2);
                debug_frame.add_point(gaze_pt, [1.0, 0.0, 0.0]);
                debug_frame.add_point(px_gaze, [1.0, 0.0, 1.0]);
                debug.send(debug_frame);
            }
            #[cfg(not(feature = "viz-2d"))]
            let _silence_warnings = (&px_gaze, &debug);
        }

        if tick_gaze {
            let dt = calc_dt(tick, &mut last_gaze_tick);
            px_gaze = vec2(raw_gaze.x * (display_width as f32),
                           raw_gaze.y * (display_height as f32));
            gaze_pt = fixation_filter.transform(px_gaze, dt);
            // println!("GAZE {:?}", gaze_pt);
        }
    }
}

fn main() {
    println!("Hello, world!");
    let (mut pool, rx) = InputPool::new();
    pool.spawn(ltr_input::listen);
    #[cfg(feature = "tobii")]
    pool.spawn(tobii_input::listen);

    #[cfg(feature = "viz-2d")]
    let (debug_view, debug_sender) = DebugWindow::new();
    #[cfg(not(feature = "viz-2d"))]
    let debug_sender = DebugSender();

    let handle = thread::spawn(|| run_pipeline(rx, debug_sender));

    #[cfg(feature = "viz-2d")]
    {
        debug_view.run();
        mem::drop(pool);
    }

    handle.join().unwrap();
}
