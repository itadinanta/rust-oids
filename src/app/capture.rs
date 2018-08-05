use app::constants::*;
use chrono::DateTime;
use chrono::Utc;
use gl;
use glutin;
use glutin::GlContext;
use image;
use image::ImageBuffer;
use num::Integer;
use rayon;
use std::fs::create_dir_all;
use std::path::PathBuf;

pub struct Capture {
	seq: usize,
	capture_path: PathBuf,
	capture_prefix: String,
	enabled: bool,
	w: u32,
	h: u32,
}

impl Capture {
	// Initializes capture system
	pub fn init(window: &glutin::GlWindow) -> Capture {
		//use gl;
		gl::ReadPixels::load_with(|s| window.get_proc_address(s) as *const _);
		let (w, h) = window.get_inner_size().unwrap();
		let now: DateTime<Utc> = Utc::now();
		Capture {
			seq: 0,
			capture_path: PathBuf::from(CAPTURE_FOLDER).join(now.format(CAPTURE_FOLDER_TIMESTAMP_PATTERN).to_string()),
			capture_prefix: String::from(CAPTURE_FILENAME_PREFIX),
			enabled: false,
			w,
			h,
		}
	}

	// Capture current framebuffer if recording is enabled
	pub fn screen_grab(&mut self) {
		if self.enabled {
			let w = self.w;
			let h = self.h;
			let mut buf: Vec<[u8; 3]> = vec![[0u8; 3]; (w * h) as usize];
			unsafe {
				gl::ReadPixels(
					0,
					0,
					w as i32,
					h as i32,
					gl::RGB,
					gl::UNSIGNED_BYTE,
					buf.as_mut_ptr() as *mut _,
				);
			}
			self.seq += 1;
			let filename = self.capture_prefix.clone() + &format!("{:08}.png", self.seq);
			let full_path = self.capture_path.join(filename);
			rayon::spawn(move || {
				// throws it into the background
				let mut img = ImageBuffer::new(w, h);
				for (idx, rgb) in (0u32..).zip(buf) {
					let (i, j) = idx.div_mod_floor(&w);
					img.put_pixel(j, h - i - 1, image::Rgb(rgb));
				}
				match img.save(full_path.clone()) {
					Ok(_) => println!("Saved image {}", full_path.to_str().unwrap()),
					Err(_) => println!("Could not save image {}", full_path.to_str().unwrap()),
				}
			});
		}
	}

	// Remote control, detects state changes
	pub fn enable(&mut self, enabled: bool) {
		if enabled != self.enabled {
			self.toggle()
		}
	}

	// Starts/restarts recording
	pub fn start(&mut self) {
		match create_dir_all(self.capture_path.clone()) {
			Ok(_) => self.enabled = true,
			Err(msg) => error!(
				"Could not create capture directory {}: {}",
				self.capture_path.to_str().unwrap(),
				msg
			),
		}
	}

	// Stops recording and flushes
	pub fn stop(&mut self) { self.enabled = false; }

	pub fn enabled(&self) -> bool { self.enabled }

	pub fn toggle(&mut self) {
		if self.enabled {
			self.stop();
		} else {
			self.start();
		}
	}
}
