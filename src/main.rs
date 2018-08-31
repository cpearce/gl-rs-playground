// Based on https://github.com/brendanzab/gl-rs/blob/master/gl/examples/triangle.rs
// and https://open.gl/drawing

// extern crate gl;
// extern crate glutin;

// use glutin::dpi::*;
// use glutin::GlContext;

// fn main() {
//     let mut events_loop = glutin::EventsLoop::new();
//     let window = glutin::WindowBuilder::new()
//         .with_title("Hello, world!")
//         .with_dimensions(LogicalSize::new(1024.0, 768.0));
//     let context = glutin::ContextBuilder::new().with_vsync(true);
//     let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

//     unsafe {
//         gl_window.make_current().unwrap();
//     }

//     unsafe {
//         gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);
//         gl::ClearColor(0.0, 0.0, 0.0, 1.0);
//     }

//     let mut running = true;
//     while running {
//         events_loop.poll_events(|event| match event {
//             glutin::Event::WindowEvent { event, .. } => match event {
//                 glutin::WindowEvent::CloseRequested => running = false,
//                 glutin::WindowEvent::Resized(logical_size) => {
//                     let dpi_factor = gl_window.get_hidpi_factor();
//                     gl_window.resize(logical_size.to_physical(dpi_factor));
//                 }
//                 glutin::WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
//                     Some(glutin::VirtualKeyCode::Escape) => running = false,
//                     _ => (),
//                 },
//                 _ => (),
//             },
//             _ => (),
//         });

//         unsafe {
//             gl::Clear(gl::COLOR_BUFFER_BIT);
//         }

//         gl_window.swap_buffers().unwrap();
//     }
// }



// Copyright 2015 Brendan Zabarauskas and the gl-rs developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate gl;
extern crate glutin;

use gl::types::*;
use std::mem;
use std::ptr;
use std::str;
use std::ffi::CString;
use std::time::Instant;

// Vertex data
static VERTEX_DATA: [GLfloat; 20] = [
    -0.5, 0.5, 1.0, 0.0, 0.0,
    0.5, 0.5, 0.0, 1.0, 0.0,
    0.5, -0.5, 0.0, 0.0, 1.0,
    -0.5, -0.5, 0.0, 0.0, 1.0,
];


static VERTEX_ELEMENTS: [GLuint; 6] = [
    0, 1, 2,
    2, 3, 0
];

// Shader sources
static VS_SRC: &'static str = "
#version 150
in vec2 position;
in vec3 color;
out vec3 Color;

void main() {
    Color = color;
    gl_Position = vec4(position, 0.0, 1.0);
}";

static FS_SRC: &'static str = "
#version 150
in vec3 Color;
out vec4 out_color;

void main() {
    out_color = vec4(Color, 1.0);
}";

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        shader = gl::CreateShader(ty);
        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(
                shader,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                str::from_utf8(&buf)
                    .ok()
                    .expect("ShaderInfoLog not valid utf8")
            );
        }
    }
    shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(
                program,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                str::from_utf8(&buf)
                    .ok()
                    .expect("ProgramInfoLog not valid utf8")
            );
        }
        program
    }
}

fn main() {
    use glutin::GlContext;

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new();
    let context = glutin::ContextBuilder::new();
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    // It is essential to make the context current before calling `gl::load_with`.
    unsafe { gl_window.make_current() }.unwrap();

    // Load the OpenGL function pointers
    // TODO: `as *const _` will not be needed once glutin is updated to the latest gl version
    gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

    // Create GLSL shaders
    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
    let program = link_program(vs, fs);

    let mut vao = 0;
    let mut vbo = 0;
    let mut ebo = 0;

    unsafe {
        // Create Vertex Array Object
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // Create a Vertex Buffer Object and copy the vertex data to it
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            mem::transmute(&VERTEX_DATA[0]),
            gl::STATIC_DRAW,
        );

        gl::GenBuffers(1, &mut ebo);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (VERTEX_ELEMENTS.len() * mem::size_of::<GLuint>()) as GLsizeiptr,
            mem::transmute(&VERTEX_ELEMENTS[0]),
            gl::STATIC_DRAW,
        );

        // Use shader program
        gl::UseProgram(program);
        gl::BindFragDataLocation(program, 0, CString::new("out_color").unwrap().as_ptr());

        // Specify the layout of the vertex data
        let pos_attr = gl::GetAttribLocation(program, CString::new("position").unwrap().as_ptr());
        gl::EnableVertexAttribArray(pos_attr as GLuint);
        gl::VertexAttribPointer(
            pos_attr as GLuint,
            2,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            5 * mem::size_of::<GLfloat>() as i32,
            ptr::null(),
        );
        let color_attr = gl::GetAttribLocation(program, CString::new("color").unwrap().as_ptr());
        gl::EnableVertexAttribArray(color_attr as GLuint);
        gl::VertexAttribPointer(
            color_attr as GLuint,
            3,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            5 * mem::size_of::<GLfloat>() as i32,
            (2 * mem::size_of::<GLfloat>()) as *const GLvoid,
        );
    }

    let start = Instant::now();
    let mut running = true;
    while running {
        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => running = false,
                glutin::WindowEvent::Resized(logical_size) => {
                    let dpi_factor = gl_window.get_hidpi_factor();
                    gl_window.resize(logical_size.to_physical(dpi_factor));
                }
                glutin::WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                    Some(glutin::VirtualKeyCode::Escape) => running = false,
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        });

        unsafe {
            // Clear the screen to black
            gl::ClearColor(0.3, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            let uni_color = gl::GetUniformLocation(program, CString::new("triangleColor").unwrap().as_ptr());
            let elapsed = Instant::now().duration_since(start);
            let t = elapsed.as_secs() as f64 + elapsed.subsec_millis() as f64 / 1000.0;
            let color = ((t * 4.0).sin() + 1.0) / 2.0;
            gl::Uniform3f(uni_color, color as f32, 0.0, 0.0);

            // Draw a triangle from the 3 vertices
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, ptr::null());
        }

        gl_window.swap_buffers().unwrap();

    }

    // Cleanup
    unsafe {
        gl::DeleteProgram(program);
        gl::DeleteShader(fs);
        gl::DeleteShader(vs);
        gl::DeleteBuffers(1, &vbo);
        gl::DeleteVertexArrays(1, &vao);
    }
}
