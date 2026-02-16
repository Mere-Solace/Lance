use std::io::Write;
use std::process::{Child, Command, Stdio};

pub struct Recorder {
    child: Child,
    width: u32,
    height: u32,
    pixel_buf: Vec<u8>,
}

impl Recorder {
    pub fn new(width: u32, height: u32, output_path: &str) -> Self {
        let size_arg = format!("{}x{}", width, height);
        let child = Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "rawvideo",
                "-pixel_format", "rgb24",
                "-video_size", &size_arg,
                "-framerate", "60",
                "-i", "pipe:0",
                "-vf", "vflip",
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-preset", "fast",
                output_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn ffmpeg — is it installed and on PATH?");

        let buf_size = (width * height * 3) as usize;

        Self {
            child,
            width,
            height,
            pixel_buf: vec![0u8; buf_size],
        }
    }

    pub fn capture_frame(&mut self) {
        unsafe {
            gl::ReadPixels(
                0,
                0,
                self.width as i32,
                self.height as i32,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                self.pixel_buf.as_mut_ptr() as *mut _,
            );
        }

        if let Some(stdin) = self.child.stdin.as_mut() {
            let _ = stdin.write_all(&self.pixel_buf);
        }
    }

    pub fn finish(mut self) {
        // Close stdin to signal EOF to ffmpeg
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}
