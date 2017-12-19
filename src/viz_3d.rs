use kiss3d::window::Window;
use kiss3d::light::Light;
use na::{Vector3, Translation3, Point3, UnitQuaternion};
use std::sync::mpsc::Receiver;
use inputs::Input;

use std::fs::File;
use std::io::Write;

pub const DEG2RAD : f32 = (2.0*3.1415926)/360.0;

pub fn run(rx: Receiver<Input>) {
    let div = 100.0;
    let monitor_width = 527.0 / div;
    let monitor_height = 296.0 / div;

    let mut window = Window::new("Kiss3d: points");
    let mut head = window.add_cube(10.0 / div, 20.0 / div, 10.0 / div);
    let mut left_eye = window.add_sphere(25.0 / div);
    let mut right_eye = window.add_sphere(25.0 / div);
    let mut gaze = window.add_sphere(10.0 / div);
    let mut screen = window.add_quad(monitor_width, monitor_height, 1, 1);
    head.set_color(1.0, 0.0, 1.0);
    left_eye.set_color(1.0, 0.0, 0.0);
    right_eye.set_color(0.0, 1.0, 0.0);
    gaze.set_color(0.,1.,1.0);
    screen.set_color(1.,1.,1.);

    window.set_light(Light::StickToCamera);

    let mut init_point = None;

    let mut data_f = File::create("data/data.csv").unwrap();
    writeln!(&mut data_f, "yaw, pitch, roll, tx, ty, tz, left_x, left_y, left_z, right_x, right_y, right_z, gaze_x, gaze_y").unwrap();
    let mut head_info = None;
    let mut gaze_info = None;

    while window.render() {
        while let Ok(input) = rx.try_recv() {
            match input {
                Input::LinuxTrackHead { yaw, pitch, roll, tx,ty,tz} => {
                    let head_offset = Translation3::new(tx / div, ty / div, tz / div);
                    head.set_local_translation(head_offset * init_point.unwrap());
                    head.set_local_rotation(UnitQuaternion::from_euler_angles(pitch*DEG2RAD, yaw*DEG2RAD, roll*DEG2RAD));
                    head_info = Some((yaw, pitch, roll, tx, ty, tz));
                }
                Input::TobiiGaze { x, y } => {
                    gaze.set_local_translation(Translation3::new((x-0.5) * monitor_width, (-y+0.5)*monitor_height, 0.0));
                    gaze_info = Some((x,y));
                }
                Input::TobiiEyePosition { left_xyz, right_xyz } => {
                    let left_valid = left_xyz != [0.0,0.0,0.0];
                    left_eye.set_visible(left_valid);
                    let left_loc = Translation3::new(left_xyz[0] / div, left_xyz[1] / div, left_xyz[2] / div);
                    left_eye.set_local_translation(left_loc);
                    let right_valid = right_xyz != [0.0,0.0,0.0];
                    right_eye.set_visible(right_valid);
                    let right_loc = Translation3::new(right_xyz[0] / div, right_xyz[1] / div, right_xyz[2] / div);
                    right_eye.set_local_translation(right_loc);

                    if init_point.is_none() && left_valid && right_valid {
                        let middle = (left_loc.vector + right_loc.vector) * 0.5;
                        init_point = Some(Translation3::from_vector(middle));
                    }

                    if let (Some((yaw, pitch, roll, tx, ty, tz)), Some((gaze_x, gaze_y))) = (head_info, gaze_info) {
                        writeln!(&mut data_f, "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",yaw, pitch, roll, tx, ty, tz, left_xyz[0], left_xyz[1], left_xyz[2], right_xyz[0], right_xyz[1], right_xyz[2], gaze_x, gaze_y).unwrap();
                    }
                }
            }
        }

        let origin = Point3::new(0.0,0.0,0.0);
        let x = Point3::new(10.0, 0.0, 0.0);
        let y = Point3::new(0.0, 10.0, 0.0);
        let z = Point3::new(0.0, 0.0, 10.0);

        window.draw_line(&origin, &x, &Point3::new(1.0, 0.0, 0.0));
        window.draw_line(&origin, &y, &Point3::new(0.0, 1.0, 0.0));
        window.draw_line(&origin, &z, &Point3::new(0.0, 0.0, 1.0));
    }
}
