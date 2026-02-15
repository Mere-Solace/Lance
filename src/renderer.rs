pub fn init() {
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::ClearColor(0.1, 0.1, 0.15, 1.0);
    }
}

pub fn begin_frame() {
    unsafe {
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }
}
