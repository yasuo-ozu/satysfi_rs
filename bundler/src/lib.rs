#[macro_use]
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
	(@default_to {$d:ty}) => { $d };
	(@default_to {$d:ty} $($t:tt)+) => { $($t)+ };
	(@emit_call $cl:ident $cr:ident $arg1:ident) => { $cl.call($cr, $arg1) };
	(@emit_call $cl:ident $cr:ident $arg1:ident $arg2:ident) => { $cl.call2($cr, $arg1, $arg2) };
	(@emit_call $cl:ident $cr:ident $arg1:ident $arg2:ident $arg3:ident) => { $cl.call3($cr, $arg1, $arg2, $arg3) };
	(@emit_call $cl:ident $cr:ident $($arg:ident)+) => {
		$cl.call_n($cr, &mut [$(unsafe { $arg.get_raw() }),+])
	};
	() => ();

	(
		$(#[ocaml_name=$ocamlname:expr])?
		$(#[doc=$doc:expr])*
		$vis:vis fn $name:ident(
			$(
				$(#[ocaml_type=$otyp:ty])?
				$arg:ident: $typ:ty
			),+ $(,)?
		) $( -> $(#[ocaml_type=$rotyp:ty])? $rtyp:ty)?; $($t:tt)*
	) => {
		$(#[doc=$doc])*
		$vis fn $name<'a>(
			$($arg: $typ),+
		) $(-> $rtyp)? {
			$crate::ocaml_interop::OCamlRuntime::init_persistent();
			$(
				let cr = unsafe { $crate::ocaml_interop::OCamlRuntime::recover_handle() };
				let $arg = <$typ as $crate::ocaml_interop::ToOCaml<$crate::ocaml_defs!(@default_to {$typ} $($otyp)?)>>::to_ocaml(&$arg, cr);
				let $arg = $arg.as_ref();
			)+
			$crate::ocaml_closure_reference!(closure, $name $(,$ocamlname)?);
			let cr = unsafe { $crate::ocaml_interop::OCamlRuntime::recover_handle() };
			let ret: $crate::ocaml_interop::OCaml<'_, $crate::ocaml_defs!(@default_to {()} $($crate::ocaml_defs!(@default_to {$rtyp} $($rotyp)?))?)>
				= $crate::ocaml_defs!(@emit_call closure cr $($arg)*);
			$(ret.to_rust::<$rtyp>())?
		}

		$crate::ocaml_defs!($($t)*);
	};

	(
		$(#[ocaml_type=$otyp:ty])?
		$(#[doc=$doc:expr])*
		$vis:vis enum $name:ident {
			$(
				$(#[doc=$idoc:expr])*
				$item:ident $(( $( $(#[ocaml_type=$iotyp:ty])? $iname:ident : $typ:ty),+ $(,)? ))?
			),+ $(,)?
		} $($t:tt)*
	) => {
		$crate::ocaml_defs!(
			@emit_enum
			{{$($otyp)?} {$($doc)*} {$vis} $name} {} {}
			$(
				$(#[doc=$idoc])*
				$item $(( $( $(#[ocaml_type=$iotyp])? $iname: $typ),+ ))?
			),+
		);
		$crate::ocaml_defs!($($t)*);
	};

	(@emit_enum {
		{$($otyp:ty)?} {$($doc:expr)*} {$vis:vis} $name:ident
	} {$(
		{{$($idoc:expr)*} $item:ident $({$($iname:ident, $typ:ty, $iotyp:ty,)+})?}
	)*} {}) => {
		$(#[doc = $doc])*
		$vis enum $name {
			$(
				$(#[doc = $idoc])*
				$item $(($($typ),+))?
			),*
		}
		$crate::ocaml_interop::impl_conv_ocaml_variant! {
			$($otyp =>)? $name {
				$(
					$name::$item $(($($iname: $iotyp),+))?
				),+
			}
		}
	};

	(
		@emit_enum {$($env:tt)*}
		{$($parsed:tt)*} {}
		$(#[doc=$idoc:expr])*
		$item:ident $(( $(,)? ))?
		$(,$($t:tt)*)?
	) => {
		$crate::ocaml_defs!(
			@emit_enum {$($env)*} {$($parsed)* {{$($idoc)*} $item }} {} $($($t)*)?
		);
	};

	(
		@emit_enum {$($env:tt)*}
		{$($parsed:tt)*} {$($parsing:tt)+}
		$(#[doc=$idoc:expr])*
		$item:ident $(( $(,)? ))?
		$(,$($t:tt)*)?
	) => {
		$crate::ocaml_defs!(
			@emit_enum {$($env)*} {$($parsed)* {{$($idoc)*} $item {$($parsing)+}}} {} $($($t)*)?
		);
	};

	(
		@emit_enum {$($env:tt)*} // with ocaml_type
		{$($parsed:tt)*} {$($parsing:tt)*}
		$(#[doc=$idoc:expr])*
		$item:ident ( #[ocaml_type=$iotyp:ty] $iname:ident : $typ:ty $(,$($other:tt)*)? )
		$($t:tt)*
	) => {
		$crate::ocaml_defs!(
			@emit_enum {$($env)*}
			{$($parsed)*} {$($parsing)* $iname, $typ, $iotyp,}
			$(#[doc=$idoc])*
			$item:ident ( $($($other:tt)*)? )
			$($t:tt)*
		);
	};

	(
		@emit_enum {$($env:tt)*} // without ocaml_type
		{$($parsed:tt)*} {$($parsing:tt)*}
		$(#[doc=$idoc:expr])*
		$item:ident ( $iname:ident : $typ:ty $(,$($other:tt)*)? )
		$($t:tt)*
	) => {
		$crate::ocaml_defs!(
			@emit_enum {$($env)*}
			{$($parsed)*} {$($parsing)* $iname, $typ, $typ,}
			$(#[doc=$idoc])*
			$item ( $($($other)*)? )
			$($t)*
		);
	};

	(
		$(#[ocaml_type=$otyp:ty])?
		$(#[doc=$doc:expr])*
		$vis:vis struct $name:ident {
			$(
				$(#[ocaml_type=$iotyp:ty])?
				$(#[doc=$idoc:expr])*
				$ivis:vis $item:ident : $typ:ty
			),+ $(,)?
		} $($t:tt)*
	) => {
		$(#[doc = $doc])*
		$vis struct $name {
			$(
				$(#[doc = $idoc])*
				$vis $item : $typ
			),+
		}
		$crate::ocaml_interop::impl_conv_ocaml_record! {
			$name $(=> $otyp)? {
				$(
					$item : $crate::ocaml_defs!(@default_to {$typ} $($iotyp)?),
				),+
			}
		}
		$crate::ocaml_defs!($($t)*);
	};
}

// ocaml_defs! {
// 	#[ocaml_name = "Main.testfn"]
// 	pub fn testfn(#[ocaml_type = String] s: &str) -> OsString;
// }

pub mod entry {

	ocaml_defs! {
		#[ocaml_name = "Main.show_error_category"]
		/// hello
		pub fn show_error_category(cat: ErrorCategory) -> String;

		#[ocaml_type = Line]
		pub enum Line {
			NormalLine(s: String),
			DisplayLine(s: String),
			NormalLineOption(o: Option<String>),
			DisplayLineOption(o: Option<String>),
		}

		pub enum ErrorCategory {
			Lexer,
			Parser,
			Typechecker,
			Evaluator,
			Interface,
			System,
		}
	}
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
