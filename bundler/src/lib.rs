// pub extern crate ocaml_interop;
pub extern crate ocaml_sys;

pub use ocaml_sys::caml_startup;
use std::ffi::c_void;

#[link(name = "satysfi")]
extern "C" {
	pub fn caml_iterate_named_values(
		f: *const extern "C" fn(*const c_void, *const std::os::raw::c_char),
	);
}

pub mod binding {
	pub struct FileDependencyGraph;
}

#[no_mangle]
extern "C" fn print_name(_val: *const c_void, name: *const std::os::raw::c_char) {
	let s = unsafe { std::ffi::CStr::from_ptr(name) };
	println!("> {:?}", s);
}

pub fn run_main() {
	unsafe {
		caml_iterate_named_values(print_name as *const _);
	}
	let arg0 = "ocaml\0".as_ptr() as *const std::os::raw::c_char;
	let c_args = vec![arg0, core::ptr::null()];
	unsafe {
		ocaml_sys::caml_startup(c_args.as_ptr());
	}
}
// nothing here
