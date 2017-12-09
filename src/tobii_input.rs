use tobii_sys::*;

use std::ptr;
use std::mem;
use std::os::raw;
use std::sync::mpsc::{SyncSender, Receiver};
use std::ffi::{CStr, CString};

use inputs::{Input, InputAction};

use tobii_sys::helpers::{self, PtrWrapper, status_to_result, TobiiError};

struct CallbackContext {
    output: SyncSender<Input>,
}

unsafe extern "C"
fn custom_log_fn(_log_context: *mut ::std::os::raw::c_void, level: LogLevel, text: *const raw::c_char) {
    if level > TOBII_LOG_LEVEL_WARN { return; }
    let s = CStr::from_ptr(text);
    println!("LOG {}: {}", level, s.to_str().unwrap());
}

unsafe extern "C"
fn gaze_callback(gaze_point: *const GazePoint, user_data: *mut ::std::os::raw::c_void) {
    assert_ne!(user_data,ptr::null_mut());
    let context = &*(user_data as *mut CallbackContext);
    let pt = &*gaze_point;
    let event = Input::TobiiGaze { x: pt.position_xy[0], y: pt.position_xy[1]};
    context.output.send(event).unwrap();
}

unsafe fn input_loop(output: SyncSender<Input>, inbox: Receiver<InputAction>) -> Result<(), TobiiError> {
    let custom_log = CustomLog {
        log_context: ptr::null_mut(),
        log_func: Some(custom_log_fn)
    };

    println!("Initializing API!");
    let mut api_ptr: *mut Api = mem::zeroed();
    let status = tobii_api_create( &mut api_ptr as *mut *mut Api, ptr::null_mut(), &custom_log as *const _);
    status_to_result(status)?;
    let api = PtrWrapper::new(api_ptr, tobii_api_destroy);

    let devices = helpers::list_devices(api.ptr())?;
    println!("{:?}", devices);

    if devices.len() < 1 {
        println!("No devices");
        return Ok(());
    }

    let url_c_string = CString::new(devices[0].clone()).unwrap();
    let url_c = url_c_string.as_c_str();
    let mut device_ptr: *mut Device = mem::zeroed();
    let status = tobii_device_create(api.ptr(), url_c.as_ptr(), &mut device_ptr as *mut *mut Device);
    status_to_result(status)?;
    let device = PtrWrapper::new(device_ptr, tobii_device_destroy);

    let mut context = Box::new(CallbackContext {output});
    let context_borrow = context.as_mut();
    let status = tobii_gaze_point_subscribe(device.ptr(), Some(gaze_callback), (context_borrow as *mut CallbackContext) as *mut raw::c_void);
    let _subscription = PtrWrapper::new(device.ptr(), tobii_gaze_point_unsubscribe);
    status_to_result(status)?;

    loop {
        match inbox.try_recv() {
            Ok(InputAction::Shutdown) => break,
            Err(_) => (),
        }

        let status = tobii_wait_for_callbacks(device.ptr());
        match status_to_result(status) {
            Err(TobiiError::TimedOut) => continue,
            Err(TobiiError::ConnectionFailed) => {
                status_to_result(helpers::reconnect(device.ptr()))?;
                continue;
            },
            Err(e) => return Err(e),
            Ok(()) => (),
        }

        let status = tobii_process_callbacks(device.ptr());
        if status == TOBII_ERROR_CONNECTION_FAILED {
            status_to_result(helpers::reconnect(device.ptr()))?;
            continue;
        }
        status_to_result(status)?;
    }
    Ok(())
}

pub fn listen(output: SyncSender<Input>, inbox: Receiver<InputAction>) {
    match unsafe { input_loop(output, inbox) } {
        Ok(()) => (),
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
