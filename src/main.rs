mod game;

extern crate crossbeam_channel;

use crossbeam_channel::unbounded;
use futures::executor::block_on;

use game::FromGameClient;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub enum EventLoopMsg {
    Stop,
}

async fn async_main() {
    let (sender_from_client_to_manager, _receiver_from_client_to_manager) =
        unbounded::<FromGameClient>();
    let (sender_to_client, receiver_to_client) = unbounded::<game::ToGameClient>();
    let _sender_to_client_from_manager = sender_to_client.clone();

    let (sender_to_event_loop, receiver_to_event_loop) = unbounded::<EventLoopMsg>();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut client = game::Client::new(
        window,
        sender_to_client,
        receiver_to_client,
        sender_to_event_loop,
        sender_from_client_to_manager,
    )
    .await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { .. } => {
            client.handle_winit_event(&event);
        }
        Event::MainEventsCleared => match receiver_to_event_loop.try_recv() {
            Ok(EventLoopMsg::Stop) => {
                println!("[DEBUG] Handling EventLoopMsg event: {:?}", event);
                *control_flow = ControlFlow::Exit;
            }
            _ => {
                client.receive();
                client.render();
            }
        },
        _ => {}
    });
}

fn main() {
    block_on(async_main());
}
