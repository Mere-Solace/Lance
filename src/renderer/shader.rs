use gl::types::*;
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;

pub struct ShaderProgram {
    pub id: GLuint,
    uniform_cache: HashMap<String, GLint>,
}

impl ShaderProgram {
    pub fn from_sources(vert_src: &str, frag_src: &str) -> Result<Self, String> {
        unsafe {
            let vert = compile_shader(vert_src, gl::VERTEX_SHADER)?;
            let frag = compile_shader(frag_src, gl::FRAGMENT_SHADER)?;

            let program = gl::CreateProgram();
            gl::AttachShader(program, vert);
            gl::AttachShader(program, frag);
            gl::LinkProgram(program);

            let mut success = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            if success == 0 {
                let mut len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buf = vec![0u8; len as usize];
                gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut _);
                buf.pop(); // remove null terminator
                let msg = String::from_utf8_lossy(&buf).to_string();
                gl::DeleteProgram(program);
                gl::DeleteShader(vert);
                gl::DeleteShader(frag);
                return Err(format!("Shader link error: {msg}"));
            }

            gl::DeleteShader(vert);
            gl::DeleteShader(frag);

            Ok(Self {
                id: program,
                uniform_cache: HashMap::new(),
            })
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    fn get_uniform_location(&mut self, name: &str) -> GLint {
        if let Some(&loc) = self.uniform_cache.get(name) {
            return loc;
        }
        let cname = CString::new(name).unwrap();
        let loc = unsafe { gl::GetUniformLocation(self.id, cname.as_ptr()) };
        self.uniform_cache.insert(name.to_string(), loc);
        loc
    }

    pub fn set_mat4(&mut self, name: &str, mat: &Mat4) {
        let loc = self.get_uniform_location(name);
        unsafe {
            gl::UniformMatrix4fv(loc, 1, gl::FALSE, mat.to_cols_array().as_ptr());
        }
    }

    pub fn set_vec3(&mut self, name: &str, v: Vec3) {
        let loc = self.get_uniform_location(name);
        unsafe {
            gl::Uniform3f(loc, v.x, v.y, v.z);
        }
    }

    pub fn set_float(&mut self, name: &str, val: f32) {
        let loc = self.get_uniform_location(name);
        unsafe {
            gl::Uniform1f(loc, val);
        }
    }

    pub fn set_vec4(&mut self, name: &str, v: [f32; 4]) {
        let loc = self.get_uniform_location(name);
        unsafe {
            gl::Uniform4f(loc, v[0], v[1], v[2], v[3]);
        }
    }

    pub fn set_int(&mut self, name: &str, val: i32) {
        let loc = self.get_uniform_location(name);
        unsafe {
            gl::Uniform1i(loc, val);
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

unsafe fn compile_shader(src: &str, shader_type: GLenum) -> Result<GLuint, String> {
    let shader = gl::CreateShader(shader_type);
    let c_src = CString::new(src).unwrap();
    gl::ShaderSource(shader, 1, &c_src.as_ptr(), ptr::null());
    gl::CompileShader(shader);

    let mut success = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
    if success == 0 {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = vec![0u8; len as usize];
        gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut _);
        buf.pop();
        let kind = if shader_type == gl::VERTEX_SHADER {
            "vertex"
        } else {
            "fragment"
        };
        let msg = String::from_utf8_lossy(&buf).to_string();
        gl::DeleteShader(shader);
        return Err(format!("{kind} shader compile error: {msg}"));
    }
    Ok(shader)
}
