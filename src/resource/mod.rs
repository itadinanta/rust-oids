use std::io;
use std::fs;

pub trait ResourceLoader<I, T> {
	fn load_resource(key: I) -> Box<[T]>;
}

struct FileSystemResourceLoader {
	root: Path,
}

impl FileSystemResourceLoader {
	fn new(root: Path) -> Self {
		FileSystemResourceLoader { root: root }
	}
}

impl ResourceLoader<&str, u8> for FileSystemResourceLoader {
	fn load_resource(key: &str) -> Box<[u8]> {
		
	}
}
