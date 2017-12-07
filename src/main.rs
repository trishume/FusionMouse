extern crate linuxtrack_sys;
extern crate cgmath;
extern crate enigo;
extern crate rand;

mod inputs;
mod ltr_input;
mod transforms;

use cgmath::{vec2, Vector2};
use enigo::{Enigo, MouseControllable};

use std::sync::mpsc::Receiver;
use std::time::Instant;

use inputs::{InputPool, Input};
use transforms::{VecOneEuroFilter, Acceleration, AccumulatingRounder};

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
    let mut raw_head_pose: Vector2<f32>;

    // pipeline state
    let mut last_tick = Instant::now();
    let mut head_filter = VecOneEuroFilter::new(6.0, 1000.0, 1.0);
    let mut last_head_pose: Option<Vector2<f32>> = None;

    let mut x_round = AccumulatingRounder::new();
    let mut y_round = AccumulatingRounder::new();

    let mut enigo = Enigo::new();

    loop {
        // update input state =========================
        match rx.recv().unwrap() {
            Input::LinuxTrackHead { yaw, pitch } => {
                raw_head_pose = vec2(yaw, pitch)*-1.0;
            }
        }

        // timing info ================================
        let tick = Instant::now();
        let dur = tick.duration_since(last_tick);
        let dt = dur.as_secs() as f32 + dur.subsec_nanos() as f32 * 1.0e-9;
        last_tick = tick;

        // compute pipeline results ===================
        let smoothed_head = head_filter.filter(raw_head_pose, dt);
        // let smoothed_head = raw_head_pose;

        let head_delta = match last_head_pose {
            Some(last_pose) => smoothed_head - last_pose,
            None => vec2(0.0,0.0),
        };
        last_head_pose = Some(smoothed_head);

        let head_cursor_move = vec2(
            accel.transform(head_delta.x, dt),
            accel.transform(head_delta.y, dt)
        );

        let rounded_move = vec2(
            x_round.round(head_cursor_move.x),
            y_round.round(head_cursor_move.y),
        );

        // do something ===============================
        enigo.mouse_move_relative(rounded_move.x, rounded_move.y);
    }
}

fn main() {
    println!("Hello, world!");
    let (mut pool, rx) = InputPool::new();
    pool.spawn(ltr_input::listen);
    run_pipeline(rx);
}
