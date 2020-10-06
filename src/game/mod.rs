use crate::EventLoopMsg;
use crossbeam_channel::unbounded;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use winit::{event::WindowEvent, window::Window};

pub enum ToGameClient {}
pub enum FromGameClient {}

pub struct Client {
    window: Window,

    sender_to_client: crossbeam_channel::Sender<ToGameClient>,
    receiver_to_client: crossbeam_channel::Receiver<ToGameClient>,
    sender_to_event_loop: crossbeam_channel::Sender<EventLoopMsg>,
    sender_from_client_to_manager: crossbeam_channel::Sender<FromGameClient>,

    receiver_notify: crossbeam_channel::Receiver<notify::Result<notify::event::Event>>,
    watcher: notify::RecommendedWatcher,

    frame_count: i32,
}

impl Client {
    pub fn new(
        window: Window,
        sender_to_client: crossbeam_channel::Sender<ToGameClient>,
        receiver_to_client: crossbeam_channel::Receiver<ToGameClient>,
        sender_to_event_loop: crossbeam_channel::Sender<EventLoopMsg>,
        sender_from_client_to_manager: crossbeam_channel::Sender<FromGameClient>,
    ) -> Client {
        println!("Client init");
        window.set_title("Huskarl");

        let (receiver_notify, watcher) = {
            let (_tx, rx) = unbounded();

            // Automatically select the best implementation for your platform.
            let mut watcher: RecommendedWatcher = Watcher::new_immediate(|res| match res {
                Ok(_) => println!("event: {:?}", res),
                Err(_) => println!("watch error: {:?}", res),
            })
            .unwrap();

            // Add a path to be watched. All files and directories at that path and
            // below will be monitored for changes.
            watcher
                .watch(std::env::current_dir().unwrap(), RecursiveMode::Recursive)
                .unwrap();
            (rx, watcher)
        };

        Client {
            window,
            sender_to_client,
            receiver_to_client,
            sender_to_event_loop,
            sender_from_client_to_manager,
            receiver_notify,
            watcher,
            frame_count: 0,
        }
    }

    pub fn handle_winit_event(&mut self, _event: &winit::event::Event<()>) {
        println!("[DEBUG] handle_winit_event {:?}", _event);
        use winit::event;

        // Low level
        match _event {
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    self.sender_to_event_loop.send(EventLoopMsg::Stop).unwrap();
                }
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(vkc),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    println!("[DEBUG] Received KeyboardInput: {:?}", vkc);
                }

                _ => {
                    println!("[WARNING] {:?}", _event);
                }
            },
            _ => (),
        }
    }

    pub fn receive(&mut self) {
        let msg: std::result::Result<
            notify::Result<notify::event::Event>,
            crossbeam_channel::TryRecvError,
        > = self.receiver_notify.try_recv();

        match msg {
            Ok(Ok(event)) => {
                log::trace!("notify {:?}", event);
            }
            _ => {}
        }
    }

    pub fn render(&mut self) {
        if self.frame_count == 1 {
            self.window.set_maximized(false);
        }
        self.frame_count += 1;
    }
}
