//
// Cloned from https://github.com/gfx-rs/gfx/blob/pre-ll/src/window/glutin/src/lib.rs
//
// Copyright 2015 The Gfx-rs Developers.
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

use gfx::memory::Typed;
use gfx::{format, handle, texture};
use gfx_device_gl as device_gl;
use gfx_device_gl::Resources as R;
use glutin;
use glutin::GlContext;
use std;

/// Initialize with a window builder.
/// Generically parametrized version over the main framebuffer format.
///
/// # Example
///
/// ```no_run
/// extern crate gfx_core;
/// extern crate gfx_device_gl;
/// extern crate gfx_window_glutin;
/// extern crate glutin;
///
/// use gfx_core::format::{DepthStencil, Rgba8};
///
/// fn main() {
///    let events_loop = glutin::EventsLoop::new();
///    let window_builder = glutin::WindowBuilder::new().with_title("Example".to_string());
///    let context = glutin::ContextBuilder::new();
///    let (window, device, factory, rtv, stv) =
///        gfx_window_glutin::init::<Rgba8, DepthStencil>(window_builder, context, &events_loop);
///
///   // your code
///    }
/// ```
pub fn init<Cf, Df>(
	window: glutin::WindowBuilder,
	context: glutin::ContextBuilder,
	events_loop: &glutin::EventsLoop,
) -> (
	glutin::GlWindow,
	device_gl::Device,
	device_gl::Factory,
	handle::RenderTargetView<R, Cf>,
	handle::DepthStencilView<R, Df>,
)
where
	Cf: format::RenderFormat,
	Df: format::DepthFormat,
{
	let (window, device, factory, color_view, ds_view) =
		init_raw(window, context, events_loop, Cf::get_format(), Df::get_format());
	(window, device, factory, Typed::new(color_view), Typed::new(ds_view))
}

/// Initialize with an existing Glutin window.
/// Generically parametrized version over the main framebuffer format.
///
/// # Example (using Piston to create the window)
///
/// ```rust,ignore
/// extern crate piston;
/// extern crate glutin_window;
/// extern crate gfx_window_glutin;
///
/// // Create window with Piston
/// let settings = piston::window::WindowSettings::new("Example", [800, 600]);
/// let mut glutin_window = glutin_window::GlutinWindow::new(&settings).unwrap();
///
/// // Initialise gfx
/// let (mut device, mut factory, main_color, main_depth) =
///     gfx_window_glutin::init_existing::<ColorFormat, DepthFormat>(&glutin_window.window);
///
/// let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
/// ```
#[allow(unused)]
pub fn init_existing<Cf, Df>(
	window: &glutin::GlWindow,
) -> (device_gl::Device, device_gl::Factory, handle::RenderTargetView<R, Cf>, handle::DepthStencilView<R, Df>)
where
	Cf: format::RenderFormat,
	Df: format::DepthFormat, {
	let (device, factory, color_view, ds_view) = init_existing_raw(window, Cf::get_format(), Df::get_format());
	(device, factory, Typed::new(color_view), Typed::new(ds_view))
}

fn get_window_dimensions(window: &glutin::GlWindow) -> texture::Dimensions {
	// https://github.com/tomaka/winit/pull/370
	#[cfg(target_os = "emscripten")]
	let (width, height) = emscripten::get_canvas_size();
	#[cfg(not(target_os = "emscripten"))]
	let (width, height) = {
		let (w, h) = window.get_inner_size().unwrap();
		(w as _, h as _)
		//		let (w, h) = window.get_inner_size().unwrap();
		//		let factor = window.hidpi_factor();
		//		((w as f32 * factor) as _, (h as f32 * factor) as _)
	};
	let aa = window.get_pixel_format().multisampling.unwrap_or(0) as texture::NumSamples;

	(width, height, 1, aa.into())
}

/// Initialize with a window builder. Raw version.
pub fn init_raw(
	window: glutin::WindowBuilder,
	mut context: glutin::ContextBuilder,
	events_loop: &glutin::EventsLoop,
	color_format: format::Format,
	ds_format: format::Format,
) -> (
	glutin::GlWindow,
	device_gl::Device,
	device_gl::Factory,
	handle::RawRenderTargetView<R>,
	handle::RawDepthStencilView<R>,
) {
	let window = {
		let color_total_bits = color_format.0.get_total_bits();
		let alpha_bits = color_format.0.get_alpha_stencil_bits();
		let depth_total_bits = ds_format.0.get_total_bits();
		let stencil_bits = ds_format.0.get_alpha_stencil_bits();

		context = context
			.with_depth_buffer(depth_total_bits - stencil_bits)
			.with_stencil_buffer(stencil_bits)
			.with_pixel_format(color_total_bits - alpha_bits, alpha_bits)
			.with_srgb(color_format.1 == format::ChannelType::Srgb);

		glutin::GlWindow::new(window, context, &events_loop).unwrap()
	};

	let (device, factory, color_view, ds_view) = init_existing_raw(&window, color_format, ds_format);

	(window, device, factory, color_view, ds_view)
}

/// Initialize with an existing Glutin window. Raw version.
pub fn init_existing_raw(
	window: &glutin::GlWindow,
	color_format: format::Format,
	ds_format: format::Format,
) -> (device_gl::Device, device_gl::Factory, handle::RawRenderTargetView<R>, handle::RawDepthStencilView<R>) {
	unsafe { window.make_current().unwrap() };
	let (device, factory) = device_gl::create(|s| window.get_proc_address(s) as *const std::os::raw::c_void);

	// create the main color/depth targets
	let dim = get_window_dimensions(window);
	let (color_view, ds_view) = device_gl::create_main_targets_raw(dim, color_format.0, ds_format.0);

	// done
	(device, factory, color_view, ds_view)
}

/// Update the internal dimensions of the main framebuffer targets. Generic
/// version over the format.
pub fn update_views<Cf, Df>(
	window: &glutin::GlWindow,
	color_view: &mut handle::RenderTargetView<R, Cf>,
	ds_view: &mut handle::DepthStencilView<R, Df>,
) where
	Cf: format::RenderFormat,
	Df: format::DepthFormat,
{
	let dim = color_view.get_dimensions();
	assert_eq!(dim, ds_view.get_dimensions());
	if let Some((cv, dv)) = update_views_raw(window, dim, Cf::get_format(), Df::get_format()) {
		*color_view = Typed::new(cv);
		*ds_view = Typed::new(dv);
	}
}

/// Return new main target views if the window resolution has changed from the
/// old dimensions.
pub fn update_views_raw(
	window: &glutin::GlWindow,
	old_dimensions: texture::Dimensions,
	color_format: format::Format,
	ds_format: format::Format,
) -> Option<(handle::RawRenderTargetView<R>, handle::RawDepthStencilView<R>)> {
	let dim = get_window_dimensions(window);
	if dim != old_dimensions {
		Some(device_gl::create_main_targets_raw(dim, color_format.0, ds_format.0))
	} else {
		None
	}
}

/// Create new main target views based on the current size of the window.
/// Best called just after a WindowResize event.
#[allow(unused)]
pub fn new_views<Cf, Df>(
	window: &glutin::GlWindow,
) -> (handle::RenderTargetView<R, Cf>, handle::DepthStencilView<R, Df>)
where
	Cf: format::RenderFormat,
	Df: format::DepthFormat, {
	let dim = get_window_dimensions(window);
	let (color_view_raw, depth_view_raw) =
		device_gl::create_main_targets_raw(dim, Cf::get_format().0, Df::get_format().0);
	(Typed::new(color_view_raw), Typed::new(depth_view_raw))
}
