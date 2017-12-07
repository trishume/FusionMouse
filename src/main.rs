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
use transforms::{VecOneEuroFilter, Acceleration, stochastic_round};

fn run_pipeline(rx: Receiver<Input>) {
    // configuration
    let accel = Acceleration {
        cd_min: 10.0,
        cd_max: 65.0,
        v_min: 0.0004,
        v_max: 0.0025,
        lambda: 1000.0,
        ratio: 0.7,
    };

    // input state
    let mut raw_head_pose: Vector2<f32>;

    // pipeline state
    let mut last_tick = Instant::now();
    let mut head_filter = VecOneEuroFilter::new(6.0, 100.0, 1.0);
    let mut last_head_pose: Option<Vector2<f32>> = None;

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

        // do something ===============================
        enigo.mouse_move_relative(
            stochastic_round(head_cursor_move.x),
            stochastic_round(head_cursor_move.y),
        );
    }
}

fn main() {
    println!("Hello, world!");
    let (mut pool, rx) = InputPool::new();
    pool.spawn(ltr_input::listen);
    run_pipeline(rx);
}
