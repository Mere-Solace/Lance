use sdl2::video::{GLContext, GLProfile, Window};
use sdl2::Sdl;

pub struct GameWindow {
    _gl_context: GLContext,
    window: Window,
}

impl GameWindow {
    pub fn new(sdl: &Sdl, title: &str, width: u32, height: u32) -> Self {
        let video = sdl.video().expect("Failed to init SDL2 video");

        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_version(3, 3);

        let window = video
            .window(title, width, height)
            .opengl()
            .position_centered()
            .build()
            .expect("Failed to create window");

        let gl_context = window
            .gl_create_context()
            .expect("Failed to create GL context");

        gl::load_with(|s| video.gl_get_proc_address(s) as *const _);

        Self {
            _gl_context: gl_context,
            window,
        }
    }

    pub fn swap(&self) {
        self.window.gl_swap_window();
    }

    pub fn aspect_ratio(&self) -> f32 {
        let (w, h) = self.window.size();
        w as f32 / h as f32
    }
}
