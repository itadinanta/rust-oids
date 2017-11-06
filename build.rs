use std::path::Path;
use std::ffi::OsStr;
use std::env;

fn main() {
	let target = env::var_os("TARGET").expect("TARGET is not defined");
	if target.to_str().expect("Invalid TARGET value").ends_with(
		"x86_64-pc-windows-msvc",
	)
	{
		let current_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
		// TODO: this doesn't work, it's currently impossible to set variables for upstream
		if env::var_os("BOX2D_INCLUDE_PATH") == None {
			eprintln!("BOX2D_INCLUDE_PATH was not set");
			let box2d_include_path = Path::new(&current_dir).join("include");
			env::set_var(
				OsStr::new("BOX2D_INCLUDE_PATH"),
				box2d_include_path.as_os_str(),
			);
			println!(
				"cargo:include={}",
				box2d_include_path.to_str().expect("Invalid library path")
			);
		}
		// Library paths can be set as linking is a downstream op
		println!(
			"cargo:rustc-link-search=native={}",
			Path::new(&current_dir).join("lib/x64").to_str().expect(
				"Invalid library path",
			)
		);
	}
}
