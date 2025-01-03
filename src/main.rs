use std::env;
use std::fs::read_to_string;
use std::time::{Duration, Instant};
use metal::*;
use objc::runtime::Object;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::macos::WindowBuilderExtMacOS,
    window::WindowBuilder,
};
use winit::platform::macos::WindowExtMacOS;
#[macro_use]
extern crate objc;

fn main() {
    // Create a winit event loop and window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Metal Triangle Example")
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .with_movable_by_window_background(true)
        .build(&event_loop)
        .unwrap();

    // Initialize Metal
    let device = Device::system_default().expect("No Metal device found");
    let layer = MetalLayer::new();
    layer.set_device(&device);
    layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
    layer.set_presents_with_transaction(false);

    unsafe {
        let ns_window: *mut Object = window.ns_window() as *mut _;
        let ns_view: *mut Object = msg_send![ns_window, contentView];
        let _: () = msg_send![ns_view, setLayer: layer.as_ref()];
        let _: () = msg_send![ns_view, setWantsLayer: true];
    }

    match env::current_dir() {
        Ok(path) => println!("현재 작업 디렉토리: {}", path.display()),
        Err(e) => println!("작업 디렉토리를 가져오지 못했습니다: {}", e),
    }

    // Create a simple vertex shader and fragment shader
    let shader_source = read_to_string("src/render.metal").expect("Failed to read render.metal file");

    // Compile the shader code
    let library = device.new_library_with_source(&shader_source, &CompileOptions::new())
        .expect("Failed to compile Metal shader");
    let vertex_function = library.get_function("vertex_main", None).unwrap();
    let fragment_function = library.get_function("fragment_main", None).unwrap();

    let vertex_descriptor = VertexDescriptor::new();
    // 위치 속성 (attribute 0)
    vertex_descriptor.attributes().object_at(0).unwrap().set_format(MTLVertexFormat::Float4);
    vertex_descriptor.attributes().object_at(0).unwrap().set_offset(0);
    vertex_descriptor.attributes().object_at(0).unwrap().set_buffer_index(0);

    // 색상 속성 (attribute 1)
    vertex_descriptor.attributes().object_at(1).unwrap().set_format(MTLVertexFormat::Float4);
    vertex_descriptor.attributes().object_at(1).unwrap().set_offset(16); // Float4는 16바이트
    vertex_descriptor.attributes().object_at(1).unwrap().set_buffer_index(0);

    // 레이아웃 설정
    vertex_descriptor.layouts().object_at(0).unwrap().set_stride(32); // Float4 두 개: 32바이트
    vertex_descriptor.layouts().object_at(0).unwrap().set_step_function(MTLVertexStepFunction::PerVertex);
    vertex_descriptor.layouts().object_at(0).unwrap().set_step_rate(1);


    // Create a render pipeline
    let pipeline_descriptor = RenderPipelineDescriptor::new();
    pipeline_descriptor.set_vertex_function(Some(&vertex_function));
    pipeline_descriptor.set_fragment_function(Some(&fragment_function));
    pipeline_descriptor.set_vertex_descriptor(Some(&vertex_descriptor));
    pipeline_descriptor.color_attachments().object_at(0).unwrap().set_pixel_format(MTLPixelFormat::BGRA8Unorm);

    let pipeline_state = device.new_render_pipeline_state(&pipeline_descriptor)
        .expect("Failed to create render pipeline state");

    // Vertex data: positions and colors
    let vertex_data: [f32; 24] = [
        0.0,  0.5, 0.0, 1.0,   1.0, 0.0, 0.0, 1.0, // Top vertex (red)
        -0.5, -0.5, 0.0, 1.0,   0.0, 1.0, 0.0, 1.0, // Bottom left vertex (green)
        0.5, -0.5, 0.0, 1.0,   0.0, 0.0, 1.0, 1.0, // Bottom right vertex (blue)
    ];

    let vertex_buffer = device.new_buffer_with_data(
        vertex_data.as_ptr() as *const _,
        (vertex_data.len() * std::mem::size_of::<f32>()) as u64,
        MTLResourceOptions::CPUCacheModeDefaultCache,
    );

    // Variables to track FPS
    let mut frame_count = 0;
    let start_time = Instant::now();
    let mut last_fps_update = Instant::now();

    // Start the event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                // 매 프레임마다 창을 다시 그리도록 요청
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let drawable = layer.next_drawable().unwrap();
                let render_pass_descriptor = RenderPassDescriptor::new();
                render_pass_descriptor
                    .color_attachments()
                    .object_at(0)
                    .unwrap()
                    .set_texture(Some(&drawable.texture()));
                render_pass_descriptor
                    .color_attachments()
                    .object_at(0)
                    .unwrap()
                    .set_load_action(MTLLoadAction::Clear);
                render_pass_descriptor
                    .color_attachments()
                    .object_at(0)
                    .unwrap()
                    .set_clear_color(MTLClearColor::new(0.0, 0.0, 0.0, 1.0));
                render_pass_descriptor
                    .color_attachments()
                    .object_at(0)
                    .unwrap()
                    .set_store_action(MTLStoreAction::Store);

                let command_queue = device.new_command_queue();
                let command_buffer = command_queue.new_command_buffer();
                let render_encoder = command_buffer.new_render_command_encoder(&render_pass_descriptor);
                render_encoder.set_render_pipeline_state(&pipeline_state);
                render_encoder.set_vertex_buffer(0, Some(&vertex_buffer), 0);
                render_encoder.draw_primitives(MTLPrimitiveType::Triangle, 0, 3);
                render_encoder.end_encoding();
                command_buffer.present_drawable(&drawable);
                command_buffer.commit();

                // FPS calculation
                frame_count += 1;
                let current_time = Instant::now();
                if current_time.duration_since(last_fps_update) >= Duration::from_secs(1) {
                    let elapsed = current_time.duration_since(start_time).as_secs_f32();
                    let fps = frame_count as f32 / elapsed;
                    println!("FPS: {:.2}", fps);
                    last_fps_update = current_time;
                    // frame_count = 0;
                }
            }
            _ => {}
        }

        window.request_redraw();
    });
}
