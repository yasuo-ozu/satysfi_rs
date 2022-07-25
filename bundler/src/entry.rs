// use crate::{init_closure, OpaqueType};
// use ocaml_sys::Value;
// use std::sync::Once;
//
// pub struct FileDependencyGraph(OpaqueType);
//
// impl FileDependencyGraph {
// 	pub fn register_library_file(&mut self, abspath_in: AbsolutePath) {
// 		static mut CL: Option<*mut Value> = None;
// 		static INIT: Once = Once::new();
// 		INIT.call_once(|| {
// 			CL = init_closure("Main.register_library_file");
// 		});
// 	}
// 	pub fn register_document_file(&mut self, abspath_in: AbsolutePath) {}
// 	pub fn register_markdown_file(&mut self, setting: &str, abspath_in:
// AbsolutePath) {} }
