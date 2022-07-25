pub extern crate ocaml_interop;
pub extern crate ocaml_sys;

use ocaml_sys::{caml_startup, Value};
use std::ffi::c_void;

#[macro_export]
macro_rules! ocaml_closure_reference {
	($var:ident, $name:ident) => {
		$crate::ocaml_closure_reference!($var, $name, stringify!($name));
	};
	($var:ident, $name:ident, $ocamlname:expr) => {
		static NAME: &str = $ocamlname;
		static mut OC: Option<$crate::ocaml_interop::internal::OCamlClosure> = None;
		static INIT: ::std::sync::Once = ::std::sync::Once::new();
		let $var = unsafe {
			INIT.call_once(|| {
				OC = $crate::ocaml_interop::internal::OCamlClosure::named(NAME);
			});
			OC.unwrap_or_else(|| panic!("OCaml closure with name '{}' not registered", NAME))
		};
	};
}

/// Another version of [`ocaml_interop::ocaml!`]. It differs
/// - supports OCaml modules
/// - Input and output types are rust-native
#[macro_export]
macro_rules! ocaml_defs {
	(@emit_call $cl:ident $cr:ident $arg1:ident) => { $cl.call($cr, $arg1) };
	(@emit_call $cl:ident $cr:ident $arg1:ident $arg2:ident) => { $cl.call2($cr, $arg1, $arg2) };
	(@emit_call $cl:ident $cr:ident $arg1:ident $arg2:ident $arg3:ident) => { $cl.call3($cr, $arg1, $arg2, $arg3) };
	(@emit_call $cl:ident $cr:ident $($arg:ident)+) => {
		$cl.call_n($cr, &mut [$(unsafe { $arg.get_raw() }),+])
	};
	() => ();

	(
		$(#[ocaml_name=$ocamlname:expr])?
		$vis:vis fn $name:ident(
			$($arg:ident: $typ:ty),+ $(,)?
		) $(-> $rtyp:ty)?; $($t:tt)*
	) => {
		$vis fn $name<'a>(
			cr: &'a mut $crate::ocaml_interop::OCamlRuntime,
			$($arg: $crate::ocaml_interop::OCamlRef<$typ>),+
		) -> $crate::ocaml_interop::BoxRoot<$crate::ocaml_interop::default_to_unit!($($rtyp)?)> {
			$crate::ocaml_closure_reference!(closure, $name $(,$ocamlname)?);
			$crate::ocaml_interop::BoxRoot::new($crate::ocaml_defs!(@emit_call closure cr $($arg)*))
		}

		$crate::ocaml_defs!($($t)*);
	}
}

ocaml_defs! {
	#[ocaml_name = "Main.testfn"]
	pub fn testfn(s: String) -> String;
}

#[cfg(target_os = "windows")]
pub mod version {
	include!(concat!(env!("OUT_DIR"), "\\version.rs"));
}

#[cfg(not(target_os = "windows"))]
pub mod version {
	include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

// #[link(name = "satysfi")]
// extern "C" {
// 	pub fn caml_iterate_named_values(
// 		f: *const extern "C" fn(*const c_void, *const std::os::raw::c_char),
// 	);
// }

// pub mod binding {
// 	pub struct FileDependencyGraph;
// }
//
// #[no_mangle]
// extern "C" fn print_name(_val: *const c_void, name: *const
// std::os::raw::c_char) { 	let s = unsafe { std::ffi::CStr::from_ptr(name) };
// 	println!("> {:?}", s);
// }
//
// fn initialize_ocaml() {
// 	static INIT: std::sync::Once = std::sync::Once::new();
//
// 	INIT.call_once(|| {
// 		let arg0 = "ocaml\0".as_ptr() as *const ocaml_sys::Char;
// 		let c_args = vec![arg0, core::ptr::null()];
// 		unsafe {
// 			ocaml_sys::caml_startup(c_args.as_ptr());
// 		}
// 	})
// }
//
// fn init_closure(name: &str) -> Option<*mut Value> {
// 	let named = unsafe {
// 		let s = std::ffi::CString::new(name).unwrap();
// 		ocaml_sys::caml_named_value(s.as_ptr())
// 	};
// 	if named.is_null() || unsafe { ocaml_sys::tag_val(*named) } !=
// ocaml_sys::CLOSURE { 		None
// 	} else {
// 		Some(unsafe { std::mem::transmute::<_, *mut Value>(named) })
// 	}
// }

pub fn run_main() {}

// struct OpaqueType(std::ffi::c_void);
//
// struct BoxRoot();
