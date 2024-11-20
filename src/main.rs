use metal::*;
use objc::runtime::Object;
use objc_foundation::NSString;
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

    // Create a simple vertex shader and fragment shader
    let source = r#"
        #include <metal_stdlib>
        using namespace metal;

        struct VertexIn {
            float4 position [[attribute(0)]];
            float4 color [[attribute(1)]];
        };

        struct VertexOut {
            float4 position [[position]];
            float4 color;
        };

        vertex VertexOut vertex_main(VertexIn in [[stage_in]]) {
            VertexOut out;
            out.position = in.position;
            out.color = in.color;
            return out;
        }

        fragment float4 fragment_main(VertexOut in [[stage_in]]) {
            return in.color;
        }
    "#;

    // Compile the shader code
    let library = device.new_library_with_source(source, &CompileOptions::new())
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

    // Start the event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
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
            }
            _ => {}
        }
    });
}
