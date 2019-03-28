use std::io;

pub trait ResourceLoader<T> {
	fn load(&self, key: &str) -> io::Result<Box<[T]>>;
}

pub mod filesystem {
	use std::fs;
	use std::io;
	use std::io::Read;
	use std::path;

	#[derive(Clone)]
	pub struct ResourceLoader {
		roots: Box<[path::PathBuf]>,
	}

	pub struct ResourceLoaderBuilder {
		roots: Vec<path::PathBuf>,
	}

	impl ResourceLoaderBuilder {
		pub fn new() -> Self { ResourceLoaderBuilder { roots: Vec::new() } }

		pub fn add(&mut self, root: &path::Path) -> &mut Self {
			self.roots.push(root.to_owned());
			self
		}

		pub fn build(&self) -> ResourceLoader { ResourceLoader { roots: self.roots.clone().into_boxed_slice() } }
	}

	impl super::ResourceLoader<u8> for ResourceLoader {
		fn load(&self, key: &str) -> io::Result<Box<[u8]>> {
			// swallow the file whole into a buffer
			fn load_from_path(path: &path::Path) -> io::Result<Box<[u8]>> {
				let mut buf = Vec::new();
				fs::File::open(path)?.read_to_end(&mut buf)?;
				Ok(buf.into_boxed_slice())
			}

			// look for the first file which exists
			match self
				.roots
				.iter()
				.map(|ref r| {
					// try all roots in order, if some has it
					let mut path = path::PathBuf::from(r);
					path.push(key);
					path
				})
				.find(|path| path.exists() && path.is_file())
			{
				// and then either read it
				Some(ref p) => load_from_path(p.as_path()),
				// or give up
				None => {
					let mut err = String::from("Resource not found in path: ");
					err.push_str(key);
					Err(io::Error::new(io::ErrorKind::Other, err))
				}
			}
		}
	}
}
