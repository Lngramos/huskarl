mod render;

use crate::EventLoopMsg;
use crossbeam_channel::unbounded;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use wgpu::TextureComponentType;
use winit::{event::WindowEvent, window::Window};

pub enum ToGameClient {}
pub enum FromGameClient {}

pub struct Client {
    window: Window,
    instance: wgpu::Instance,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    sender_to_client: crossbeam_channel::Sender<ToGameClient>,
    receiver_to_client: crossbeam_channel::Receiver<ToGameClient>,
    sender_to_event_loop: crossbeam_channel::Sender<EventLoopMsg>,
    sender_from_client_to_manager: crossbeam_channel::Sender<FromGameClient>,

    receiver_notify: crossbeam_channel::Receiver<notify::Result<notify::event::Event>>,
    watcher: notify::RecommendedWatcher,

    frame_count: i32,

    render_pipeline: wgpu::RenderPipeline,
}

impl Client {
    pub async fn new(
        window: Window,
        sender_to_client: crossbeam_channel::Sender<ToGameClient>,
        receiver_to_client: crossbeam_channel::Receiver<ToGameClient>,
        sender_to_event_loop: crossbeam_channel::Sender<EventLoopMsg>,
        sender_from_client_to_manager: crossbeam_channel::Sender<FromGameClient>,
    ) -> Client {
        println!("Client init");
        window.set_title("Huskarl");

        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropiate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .expect("Failed to create device");

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

        // Load the shaders from disk
        let vs_module = device.create_shader_module(wgpu::include_spirv!("shader.vert.spv"));
        let fs_module = device.create_shader_module(wgpu::include_spirv!("shader.frag.spv"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            // Use the default rasterizer state: no culling, no depth bias
            rasterization_state: None,
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::TextureFormat::Bgra8UnormSrgb.into()],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Client {
            window,
            instance,
            size,
            surface,
            adapter,
            device,
            queue,
            render_pipeline,
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
}
