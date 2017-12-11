extern crate linuxtrack_sys;
extern crate tobii_sys;
extern crate cgmath;
extern crate enigo;
extern crate signpost;

mod inputs;
mod ltr_input;
mod tobii_input;
mod transforms;

use cgmath::{vec2, Vector2};
use enigo::{Enigo, MouseControllable};

use std::sync::mpsc::Receiver;
use std::time::Instant;
use std::mem;

use inputs::{InputPool, Input};
use transforms::{VecOneEuroFilter, Acceleration, AccumulatingRounder, FixationFilter};

fn calc_dt(tick: Instant, last_tick: &mut Instant) -> f32 {
    let dur = tick.duration_since(*last_tick);
    let dt = dur.as_secs() as f32 + dur.subsec_nanos() as f32 * 1.0e-9;
    mem::replace(last_tick, tick);
    dt
}

fn run_pipeline(rx: Receiver<Input>) {
    // configuration
    let accel = Acceleration {
        cd_min: 8.0, // min gain
        cd_max: 65.0, // max gain
        v_min: 0.0004, // input velocity lower bound
        v_max: 0.0025, // input velocity upper bound
        lambda: 1000.0, // slope of curve at inflection point
        ratio: 0.7, // where inflection lies between v_min and v_max
    };

    // input state
    let mut raw_head_pose: Vector2<f32> = vec2(0.0, 0.0);
    let mut raw_gaze: Vector2<f32> = vec2(0.0, 0.0);

    // pipeline state
    let mut last_head_tick = Instant::now();
    let mut last_gaze_tick = Instant::now();
    let mut head_filter = VecOneEuroFilter::new(6.0, 1000.0, 1.0);
    let mut last_head_pose: Option<Vector2<f32>> = None;

    let mut x_round = AccumulatingRounder::new();
    let mut y_round = AccumulatingRounder::new();

    let mut fixation_filter = FixationFilter::new(0.03, 150.0);
    let mut gaze_pt: Vector2<f32> = vec2(0.0, 0.0);

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
            Input::TobiiGaze { x, y } => {
                raw_gaze = vec2(x, y);
                tick_gaze = true;
            }
        }

        let tick = Instant::now();
        let _signpost = signpost::AutoTrace::new(1, &[0, 0, 0, signpost::Color::Blue as usize]);

        // compute pipeline results ===================
        let mut rounded_move = vec2(0i32, 0i32);
        if tick_head {
            let dt = calc_dt(tick, &mut last_head_tick);
            // println!("dt: {}", dt);
            let smoothed_head = head_filter.filter(raw_head_pose, dt);
            // let smoothed_head = raw_head_pose;

            let head_delta = match last_head_pose {
                Some(last_pose) => smoothed_head - last_pose,
                None => vec2(0.0, 0.0),
            };
            last_head_pose = Some(smoothed_head);

            let head_cursor_move = vec2(accel.transform(head_delta.x, dt),
                                        accel.transform(head_delta.y, dt));

            rounded_move = vec2(x_round.round(head_cursor_move.x),
                                y_round.round(head_cursor_move.y));
        }

        if tick_gaze {
            let dt = calc_dt(tick, &mut last_gaze_tick);
            let (display_width, display_height) = Enigo::main_display_size();
            let px_gaze = vec2(raw_gaze.x * (display_width as f32),
                               raw_gaze.y * (display_height as f32));
            gaze_pt = fixation_filter.transform(px_gaze, dt);
            // println!("GAZE {:?}", gaze_pt);
        }

        // do something ===============================
        if rounded_move.x.abs() > 0 || rounded_move.y.abs() > 0 {
            enigo.mouse_move_relative(rounded_move.x, rounded_move.y);
        }
        // enigo.mouse_move_to(gaze_pt.x as i32, gaze_pt.y as i32);
    }
}

fn main() {
    println!("Hello, world!");
    let (mut pool, rx) = InputPool::new();
    pool.spawn(ltr_input::listen);
    pool.spawn(tobii_input::listen);
    run_pipeline(rx);
}
