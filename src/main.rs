#[warn(unused_must_use)]
#[warn(unused_mut)]
extern crate xcb;

use std::process::Command;
use xcb::x;

const MODKEY: x::ModMask = x::ModMask::N4;

fn main() -> xcb::Result<()> {
    let mut wm = WindowManager::setup();
    wm.run()
}

fn spawn_alacritty() {
    Command::new("alacritty")
        .spawn()
        .expect("spawn child process failed");
}

struct WindowManager {
    running: bool,
    conn: xcb::Connection,
    root: x::Window,
}

impl WindowManager {
    fn setup() -> Self {
        let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
        let setup = conn.get_setup();
        let screen = setup.roots().nth(screen_num as usize).unwrap();
        let root = screen.root();

        conn.send_request(&x::UngrabKey {
            key: x::GRAB_ANY,
            grab_window: root,
            modifiers: x::ModMask::ANY,
        });

        conn.send_request(&x::GrabKey {
            owner_events: true,
            grab_window: root,
            modifiers: MODKEY,
            key: 24,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
        });

        conn.send_request(&x::GrabButton {
            owner_events: false,
            grab_window: root,
            event_mask: x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
            confine_to: root,
            cursor: x::CURSOR_NONE,
            button: x::ButtonIndex::N1,
            modifiers: MODKEY,
        });

        conn.send_request(&x::GrabButton {
            owner_events: false,
            grab_window: root,
            event_mask: x::EventMask::BUTTON_PRESS | x::EventMask::BUTTON_RELEASE,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
            confine_to: root,
            cursor: x::CURSOR_NONE,
            button: x::ButtonIndex::N3,
            modifiers: MODKEY,
        });

        spawn_alacritty();

        Self {
            running: false,
            conn: conn,
            root: root,
        }
    }

    pub fn run(&mut self) -> xcb::Result<()> {
        self.running = true;
        self.conn.flush()?;
        let mut current_window: x::Window = self.root;
        let mut move_by_motion = false;
        let mut resize_by_motion = false;

        while self.running {
            let event = match self.conn.wait_for_event() {
                Ok(event) => event,
                _ => continue,
            };

            match event {
                xcb::Event::X(x::Event::KeyPress(ev)) => {
                    let keycode = ev.detail();
                    println!("KeyPress {}", keycode);
                }

                xcb::Event::X(x::Event::ButtonPress(ev)) => {
                    let buttoncode = ev.detail();
                    println!("ButtonPress {}", buttoncode);

                    current_window = ev.child();
                    self.conn.send_request(&x::GrabPointer {
                        owner_events: false,
                        grab_window: self.root,
                        event_mask: x::EventMask::BUTTON_RELEASE
                            | x::EventMask::BUTTON_MOTION
                            | x::EventMask::POINTER_MOTION,
                        pointer_mode: x::GrabMode::Async,
                        keyboard_mode: x::GrabMode::Async,
                        confine_to: self.root,
                        cursor: x::CURSOR_NONE,
                        time: x::CURRENT_TIME,
                    });
                    if buttoncode == 1 {}

                    match buttoncode {
                        1 => move_by_motion = true,
                        3 => resize_by_motion = true,
                        _ => (),
                    }
                }

                xcb::Event::X(x::Event::ButtonRelease(ev)) => {
                    println!("ButtonRelease {}", ev.detail());
                    move_by_motion = false;
                    resize_by_motion = false;
                    self.conn.send_request(&x::UngrabPointer {
                        time: x::CURRENT_TIME,
                    });
                }

                xcb::Event::X(x::Event::MotionNotify(ev)) => {
                    println!(
                        "MotionNotify {} {} {} {} {} {}",
                        move_by_motion,
                        resize_by_motion,
                        ev.root_x(),
                        ev.root_y(),
                        ev.event_x(),
                        ev.event_y()
                    );
                    if move_by_motion {
                        self.conn.send_request(&x::ConfigureWindow {
                            window: current_window,
                            value_list: &[
                                x::ConfigWindow::X(ev.root_x().into()),
                                x::ConfigWindow::Y(ev.root_y().into()),
                            ],
                        });
                    } else if resize_by_motion {
                        self.conn.send_request(&x::ConfigureWindow {
                            window: current_window,
                            value_list: &[
                                x::ConfigWindow::Width(ev.root_x() as u32),
                                x::ConfigWindow::Height(ev.root_y() as u32),
                            ],
                        });
                    }
                }

                _ => continue,
            }
            self.conn.flush()?;
        }
        Ok(())
    }
}
