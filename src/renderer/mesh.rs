use gl::types::*;
use std::f32::consts::PI;
use std::mem;
use std::ptr;

pub struct Mesh {
    vao: GLuint,
    vbo: GLuint,
    ebo: GLuint,
    pub index_count: i32,
}

impl Mesh {
    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawElements(gl::TRIANGLES, self.index_count, gl::UNSIGNED_INT, ptr::null());
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
        }
    }
}

fn upload_mesh(vertices: &[f32], indices: &[u32]) -> Mesh {
    let mut vao = 0;
    let mut vbo = 0;
    let mut ebo = 0;

    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenBuffers(1, &mut ebo);

        gl::BindVertexArray(vao);

        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * mem::size_of::<f32>()) as GLsizeiptr,
            vertices.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (indices.len() * mem::size_of::<u32>()) as GLsizeiptr,
            indices.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        let stride = 6 * mem::size_of::<f32>() as GLsizei;

        // position attribute (location 0)
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());
        gl::EnableVertexAttribArray(0);

        // normal attribute (location 1)
        gl::VertexAttribPointer(
            1,
            3,
            gl::FLOAT,
            gl::FALSE,
            stride,
            (3 * mem::size_of::<f32>()) as *const _,
        );
        gl::EnableVertexAttribArray(1);

        gl::BindVertexArray(0);
    }

    Mesh {
        vao,
        vbo,
        ebo,
        index_count: indices.len() as i32,
    }
}

pub fn create_sphere(radius: f32, stacks: u32, sectors: u32) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=stacks {
        let stack_angle = PI / 2.0 - (i as f32) * PI / (stacks as f32);
        let xy = radius * stack_angle.cos();
        let z = radius * stack_angle.sin();

        for j in 0..=sectors {
            let sector_angle = 2.0 * PI * (j as f32) / (sectors as f32);
            let x = xy * sector_angle.cos();
            let y = xy * sector_angle.sin();

            // position
            vertices.push(x);
            vertices.push(z);
            vertices.push(y);

            // normal (unit sphere)
            let nx = stack_angle.cos() * sector_angle.cos();
            let ny = stack_angle.sin();
            let nz = stack_angle.cos() * sector_angle.sin();
            vertices.push(nx);
            vertices.push(ny);
            vertices.push(nz);
        }
    }

    for i in 0..stacks {
        for j in 0..sectors {
            let first = i * (sectors + 1) + j;
            let second = first + sectors + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(first + 1);
            indices.push(second);
            indices.push(second + 1);
        }
    }

    upload_mesh(&vertices, &indices)
}

pub fn create_ground_plane(half_extent: f32) -> Mesh {
    let h = half_extent;
    #[rustfmt::skip]
    let vertices: Vec<f32> = vec![
        // pos              // normal
        -h, 0.0, -h,       0.0, 1.0, 0.0,
         h, 0.0, -h,       0.0, 1.0, 0.0,
         h, 0.0,  h,       0.0, 1.0, 0.0,
        -h, 0.0,  h,       0.0, 1.0, 0.0,
    ];
    let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

    upload_mesh(&vertices, &indices)
}
