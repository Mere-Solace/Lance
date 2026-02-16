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

pub fn create_capsule(radius: f32, height: f32, sectors: u32, stacks: u32) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let half_height = height * 0.5;
    let half_stacks = stacks / 2;

    // Top hemisphere (offset up by half_height)
    for i in 0..=half_stacks {
        let stack_angle = PI / 2.0 - (i as f32) * (PI / 2.0) / (half_stacks as f32);
        let xy = radius * stack_angle.cos();
        let y = radius * stack_angle.sin() + half_height;

        for j in 0..=sectors {
            let sector_angle = 2.0 * PI * (j as f32) / (sectors as f32);
            let x = xy * sector_angle.cos();
            let z = xy * sector_angle.sin();

            vertices.push(x);
            vertices.push(y);
            vertices.push(z);

            let nx = stack_angle.cos() * sector_angle.cos();
            let ny = stack_angle.sin();
            let nz = stack_angle.cos() * sector_angle.sin();
            vertices.push(nx);
            vertices.push(ny);
            vertices.push(nz);
        }
    }

    let top_rows = half_stacks + 1;

    // Bottom hemisphere (offset down by half_height)
    for i in 0..=half_stacks {
        let stack_angle = -(i as f32) * (PI / 2.0) / (half_stacks as f32);
        let xy = radius * stack_angle.cos();
        let y = radius * stack_angle.sin() - half_height;

        for j in 0..=sectors {
            let sector_angle = 2.0 * PI * (j as f32) / (sectors as f32);
            let x = xy * sector_angle.cos();
            let z = xy * sector_angle.sin();

            vertices.push(x);
            vertices.push(y);
            vertices.push(z);

            let nx = stack_angle.cos() * sector_angle.cos();
            let ny = stack_angle.sin();
            let nz = stack_angle.cos() * sector_angle.sin();
            vertices.push(nx);
            vertices.push(ny);
            vertices.push(nz);
        }
    }

    let total_rows = top_rows + half_stacks + 1;

    // Generate indices for all rows
    for i in 0..(total_rows - 1) {
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

#[allow(dead_code)]
pub fn create_box(width: f32, height: f32, depth: f32) -> Mesh {
    let hw = width * 0.5;
    let hh = height * 0.5;
    let hd = depth * 0.5;

    #[rustfmt::skip]
    let vertices: Vec<f32> = vec![
        // Front face (+Z)
        -hw, -hh,  hd,  0.0,  0.0,  1.0,
         hw, -hh,  hd,  0.0,  0.0,  1.0,
         hw,  hh,  hd,  0.0,  0.0,  1.0,
        -hw,  hh,  hd,  0.0,  0.0,  1.0,
        // Back face (-Z)
         hw, -hh, -hd,  0.0,  0.0, -1.0,
        -hw, -hh, -hd,  0.0,  0.0, -1.0,
        -hw,  hh, -hd,  0.0,  0.0, -1.0,
         hw,  hh, -hd,  0.0,  0.0, -1.0,
        // Top face (+Y)
        -hw,  hh,  hd,  0.0,  1.0,  0.0,
         hw,  hh,  hd,  0.0,  1.0,  0.0,
         hw,  hh, -hd,  0.0,  1.0,  0.0,
        -hw,  hh, -hd,  0.0,  1.0,  0.0,
        // Bottom face (-Y)
        -hw, -hh, -hd,  0.0, -1.0,  0.0,
         hw, -hh, -hd,  0.0, -1.0,  0.0,
         hw, -hh,  hd,  0.0, -1.0,  0.0,
        -hw, -hh,  hd,  0.0, -1.0,  0.0,
        // Right face (+X)
         hw, -hh,  hd,  1.0,  0.0,  0.0,
         hw, -hh, -hd,  1.0,  0.0,  0.0,
         hw,  hh, -hd,  1.0,  0.0,  0.0,
         hw,  hh,  hd,  1.0,  0.0,  0.0,
        // Left face (-X)
        -hw, -hh, -hd, -1.0,  0.0,  0.0,
        -hw, -hh,  hd, -1.0,  0.0,  0.0,
        -hw,  hh,  hd, -1.0,  0.0,  0.0,
        -hw,  hh, -hd, -1.0,  0.0,  0.0,
    ];

    let mut indices = Vec::new();
    for face in 0..6u32 {
        let base = face * 4;
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    upload_mesh(&vertices, &indices)
}

#[allow(dead_code)]
pub fn create_cylinder(radius: f32, height: f32, segments: u32) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let half_h = height * 0.5;

    // Side vertices: two rings (top and bottom) with outward normals
    for i in 0..=segments {
        let angle = 2.0 * PI * (i as f32) / (segments as f32);
        let nx = angle.cos();
        let nz = angle.sin();
        let x = radius * nx;
        let z = radius * nz;

        // Bottom ring
        vertices.extend_from_slice(&[x, -half_h, z, nx, 0.0, nz]);
        // Top ring
        vertices.extend_from_slice(&[x, half_h, z, nx, 0.0, nz]);
    }

    // Side indices
    for i in 0..segments {
        let bot = i * 2;
        let top = bot + 1;
        let next_bot = bot + 2;
        let next_top = bot + 3;
        indices.extend_from_slice(&[bot, next_bot, top, top, next_bot, next_top]);
    }

    // Top cap
    let top_center = vertices.len() as u32 / 6;
    vertices.extend_from_slice(&[0.0, half_h, 0.0, 0.0, 1.0, 0.0]);
    let top_ring_start = vertices.len() as u32 / 6;
    for i in 0..=segments {
        let angle = 2.0 * PI * (i as f32) / (segments as f32);
        let x = radius * angle.cos();
        let z = radius * angle.sin();
        vertices.extend_from_slice(&[x, half_h, z, 0.0, 1.0, 0.0]);
    }
    for i in 0..segments {
        indices.extend_from_slice(&[top_center, top_ring_start + i, top_ring_start + i + 1]);
    }

    // Bottom cap
    let bot_center = vertices.len() as u32 / 6;
    vertices.extend_from_slice(&[0.0, -half_h, 0.0, 0.0, -1.0, 0.0]);
    let bot_ring_start = vertices.len() as u32 / 6;
    for i in 0..=segments {
        let angle = 2.0 * PI * (i as f32) / (segments as f32);
        let x = radius * angle.cos();
        let z = radius * angle.sin();
        vertices.extend_from_slice(&[x, -half_h, z, 0.0, -1.0, 0.0]);
    }
    for i in 0..segments {
        indices.extend_from_slice(&[bot_center, bot_ring_start + i + 1, bot_ring_start + i]);
    }

    upload_mesh(&vertices, &indices)
}

/// Create a sword mesh composed of blade (box), crossguard (box), and handle (cylinder).
/// Origin is at the grip point (top of handle / base of blade).
pub fn create_sword() -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Helper: append a box at an offset position, return vertex count added
    let add_box = |verts: &mut Vec<f32>, idxs: &mut Vec<u32>,
                       w: f32, h: f32, d: f32, offset_y: f32| {
        let base = verts.len() as u32 / 6;
        let hw = w * 0.5;
        let hh = h * 0.5;
        let hd = d * 0.5;
        let oy = offset_y;

        #[rustfmt::skip]
        let box_verts: [f32; 144] = [
            // Front (+Z)
            -hw, -hh + oy,  hd,  0.0,  0.0,  1.0,
             hw, -hh + oy,  hd,  0.0,  0.0,  1.0,
             hw,  hh + oy,  hd,  0.0,  0.0,  1.0,
            -hw,  hh + oy,  hd,  0.0,  0.0,  1.0,
            // Back (-Z)
             hw, -hh + oy, -hd,  0.0,  0.0, -1.0,
            -hw, -hh + oy, -hd,  0.0,  0.0, -1.0,
            -hw,  hh + oy, -hd,  0.0,  0.0, -1.0,
             hw,  hh + oy, -hd,  0.0,  0.0, -1.0,
            // Top (+Y)
            -hw,  hh + oy,  hd,  0.0,  1.0,  0.0,
             hw,  hh + oy,  hd,  0.0,  1.0,  0.0,
             hw,  hh + oy, -hd,  0.0,  1.0,  0.0,
            -hw,  hh + oy, -hd,  0.0,  1.0,  0.0,
            // Bottom (-Y)
            -hw, -hh + oy, -hd,  0.0, -1.0,  0.0,
             hw, -hh + oy, -hd,  0.0, -1.0,  0.0,
             hw, -hh + oy,  hd,  0.0, -1.0,  0.0,
            -hw, -hh + oy,  hd,  0.0, -1.0,  0.0,
            // Right (+X)
             hw, -hh + oy,  hd,  1.0,  0.0,  0.0,
             hw, -hh + oy, -hd,  1.0,  0.0,  0.0,
             hw,  hh + oy, -hd,  1.0,  0.0,  0.0,
             hw,  hh + oy,  hd,  1.0,  0.0,  0.0,
            // Left (-X)
            -hw, -hh + oy, -hd, -1.0,  0.0,  0.0,
            -hw, -hh + oy,  hd, -1.0,  0.0,  0.0,
            -hw,  hh + oy,  hd, -1.0,  0.0,  0.0,
            -hw,  hh + oy, -hd, -1.0,  0.0,  0.0,
        ];

        verts.extend_from_slice(&box_verts);
        for face in 0..6u32 {
            let b = base + face * 4;
            idxs.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        }
    };

    // Helper: append a cylinder at an offset position
    let add_cylinder = |verts: &mut Vec<f32>, idxs: &mut Vec<u32>,
                        radius: f32, height: f32, segments: u32, offset_y: f32| {
        let base = verts.len() as u32 / 6;
        let half_h = height * 0.5;

        // Side rings
        for i in 0..=segments {
            let angle = 2.0 * PI * (i as f32) / (segments as f32);
            let nx = angle.cos();
            let nz = angle.sin();
            let x = radius * nx;
            let z = radius * nz;
            verts.extend_from_slice(&[x, -half_h + offset_y, z, nx, 0.0, nz]);
            verts.extend_from_slice(&[x, half_h + offset_y, z, nx, 0.0, nz]);
        }
        for i in 0..segments {
            let bot = base + i * 2;
            let top = bot + 1;
            let next_bot = bot + 2;
            let next_top = bot + 3;
            idxs.extend_from_slice(&[bot, next_bot, top, top, next_bot, next_top]);
        }

        // Top cap
        let tc = verts.len() as u32 / 6;
        verts.extend_from_slice(&[0.0, half_h + offset_y, 0.0, 0.0, 1.0, 0.0]);
        let tr = verts.len() as u32 / 6;
        for i in 0..=segments {
            let angle = 2.0 * PI * (i as f32) / (segments as f32);
            verts.extend_from_slice(&[radius * angle.cos(), half_h + offset_y, radius * angle.sin(), 0.0, 1.0, 0.0]);
        }
        for i in 0..segments {
            idxs.extend_from_slice(&[tc, tr + i, tr + i + 1]);
        }

        // Bottom cap
        let bc = verts.len() as u32 / 6;
        verts.extend_from_slice(&[0.0, -half_h + offset_y, 0.0, 0.0, -1.0, 0.0]);
        let br = verts.len() as u32 / 6;
        for i in 0..=segments {
            let angle = 2.0 * PI * (i as f32) / (segments as f32);
            verts.extend_from_slice(&[radius * angle.cos(), -half_h + offset_y, radius * angle.sin(), 0.0, -1.0, 0.0]);
        }
        for i in 0..segments {
            idxs.extend_from_slice(&[bc, br + i + 1, br + i]);
        }
    };

    // Handle: cylinder, radius 0.02, height 0.15, centered below origin
    add_cylinder(&mut vertices, &mut indices, 0.02, 0.15, 8, -0.075);

    // Crossguard: wide short box at origin (grip point)
    add_box(&mut vertices, &mut indices, 0.2, 0.03, 0.03, 0.0);

    // Blade: tall thin box above crossguard
    add_box(&mut vertices, &mut indices, 0.05, 0.8, 0.02, 0.415);

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
