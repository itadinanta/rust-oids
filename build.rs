use std::env;
use std::path::Path;

fn main() {
	let target = env::var_os("TARGET").expect("TARGET is not defined");
	if target.to_str().expect("Invalid TARGET value").ends_with("x86_64-pc-windows-msvc") {
		let current_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
		// Library paths can be set as linking is a downstream op
		println!(
			"cargo:rustc-link-search=native={}",
			Path::new(&current_dir).join("lib/x64").to_str().expect("Invalid library path")
		)
	}
}
