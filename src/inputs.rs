use std::sync::mpsc;
use std::thread;

#[derive(Clone,Debug)]
pub enum Input {
    LinuxTrackHead { yaw: f32, pitch: f32, roll: f32, tx: f32, ty: f32, tz: f32 },
    TobiiGaze { x: f32, y: f32 },
    TobiiEyePosition {
        left_xyz: [f32; 3],
        right_xyz: [f32; 3],
    }
}

pub enum InputAction {
    // Pause,
    // Resume,
    Shutdown,
}

struct InputThread {
    inbox: mpsc::Sender<InputAction>,
    handle: Option<thread::JoinHandle<()>>,
}

pub struct InputPool {
    threads: Vec<InputThread>,
    sender: mpsc::SyncSender<Input>,
}

impl InputPool {
    pub fn new() -> (InputPool, mpsc::Receiver<Input>) {
        let (tx, rx) = mpsc::sync_channel::<Input>(50); // TODO choose best constant
        let pool = InputPool {
            threads: vec![],
            sender: tx,
        };
        (pool, rx)
    }

    pub fn spawn<F>(&mut self, f: F)
        where F: FnOnce(mpsc::SyncSender<Input>, mpsc::Receiver<InputAction>) -> (),
              F: Send + 'static
    {
        let (tx, rx) = mpsc::channel::<InputAction>();
        let sender = self.sender.clone();
        let handle = thread::spawn(|| f(sender, rx));
        self.threads
            .push(InputThread {
                      inbox: tx,
                      handle: Some(handle),
                  });
    }
}

impl Drop for InputPool {
    fn drop(&mut self) {
        for thread in &self.threads {
            thread.inbox.send(InputAction::Shutdown).unwrap();
        }

        for thread in &mut self.threads {
            if let Some(handle) = thread.handle.take() {
                handle.join().unwrap();
            }
        }
    }
}
