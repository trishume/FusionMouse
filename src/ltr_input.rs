use linuxtrack_sys::*;

use std::ptr;
use std::mem;
use std::os::raw;
use std::sync::mpsc::{SyncSender, Receiver};

use inputs::{Input, InputAction};
use signpost;

unsafe fn get_one_pose() -> Result<Pose, Status> {
    let res = linuxtrack_wait(1000); // 1 second timeout
    // println!("waited  {:?}", res);
    if res != 1 {
        let status = linuxtrack_get_tracking_state();
        println!("Status: {:?}", status);
        return Err(status);
    }
    signpost::start(3, &[0,0,0, signpost::Color::Green as usize]);

    let mut pose: Pose = mem::zeroed();
    let mut blobs: [f32; 9] = [0.0; 9];
    let mut blobs_read: raw::c_int = 0;
    let res = linuxtrack_get_pose_full(&mut pose as *mut _,
                                       blobs[..].as_mut_ptr(),
                                       3,
                                       &mut blobs_read as *mut _);
    // println!("got pose {:?}", res);
    // println!("Pose: {:?}", pose);
    // println!("Blobs: {:?}", blobs_read);

    if res != 1 || blobs_read < 3 {
        return Err(STATUS_RUNNING);
    }
    return Ok(pose);
}

unsafe fn input_loop(output: SyncSender<Input>, inbox: Receiver<InputAction>) {
    loop {
        match inbox.try_recv() {
            Ok(InputAction::Shutdown) => return,
            // Ok(InputAction::Pause) | Ok(InputAction::Resume) => unimplemented!(),
            Err(_) => (),
        }

        let pose = match get_one_pose() {
            Ok(pose) => pose,
            Err(_) => continue,
        };

        let input = Input::LinuxTrackHead {
            yaw: pose.raw_yaw,
            pitch: pose.raw_pitch,
        };
        output
            .send(input)
            .expect("shutdown should come before channel close");
        signpost::end(3, &[0,0,0, signpost::Color::Green as usize]);
    }
}

pub fn listen(output: SyncSender<Input>, inbox: Receiver<InputAction>) {
    unsafe {
        let status = linuxtrack_init(ptr::null());
        println!("Init status: {:?}", status);
        let status = linuxtrack_notification_on();
        println!("Notification status: {:?}", status);

        input_loop(output, inbox);

        linuxtrack_shutdown();
    }
}
