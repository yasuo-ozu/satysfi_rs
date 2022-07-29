#![feature(log_syntax)]
#[macro_use]
pub extern crate ocaml_interop;
pub extern crate ocaml_sys;

use ocaml_sys::{caml_startup, Value};
use std::ffi::c_void;
pub mod closure;

#[macro_export]
macro_rules! ocaml_closure_reference {
	(@noarg $var:ident, $name:ident) => {
		$crate::ocaml_closure_reference!(@noarg $var, $name, stringify!($name));
	};
	(@noarg $var:ident, $name:ident, $ocamlname:expr) => {
		{
			let name = std::ffi::CString::new($ocamlname).unwrap();
			let named = unsafe {
				$crate::ocaml_sys::caml_named_value(name.as_ptr())
			};
			if named.is_null() || unsafe { $crate::ocaml_sys::tag_val(*named) } != $crate::ocaml_sys::CLOSURE {
				panic!("Name {} not valid", $ocamlname);
			} else {
				unsafe{ *named }
			}
		}
	};
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

	// fn
	(
		$(#[ocaml_name=$ocamlname:expr])?
		$(#[doc=$doc:expr])*
		$vis:vis fn $name:ident() $( -> $(#[ocaml_type=$rotyp:ty])? $rtyp:ty)?; $($t:tt)*
	) => {
		$(#[doc=$doc])*
		$vis fn $name<'a>() $(-> $rtyp)? {
			$crate::ocaml_interop::OCamlRuntime::init_persistent();
			let val = $crate::ocaml_closure_reference!(@noarg closure, $name $(,$ocamlname)?);
			let cr = unsafe { $crate::ocaml_interop::OCamlRuntime::recover_handle() };
			let val = unsafe { $crate::ocaml_interop::OCaml::new(cr, val) };
			$(val.to_rust::<$rtyp>())?
		}

		$crate::ocaml_defs!($($t)*);
	};
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
			#[allow(unused)]
			let ret: $crate::ocaml_interop::OCaml<'_, $crate::ocaml_defs!(@default_to {()} $($crate::ocaml_defs!(@default_to {$rtyp} $($rotyp)?))?)>
				= $crate::ocaml_defs!(@emit_call closure cr $($arg)*);
			$(ret.to_rust::<$rtyp>())?
		}

		$crate::ocaml_defs!($($t)*);
	};

	// enum
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
			$({{$item $($idoc)*} $($({$iname, $typ $(,$iotyp)?})+)?})+
		);
		$crate::ocaml_defs!($($t)*);
	};

	(@emit_enum {{$($otyp:ty)?} {$($doc:expr)*} {$vis:vis} $name:ident}
	 {$({{$item:ident $($idoc:expr)*} $({$($iname:ident, $typ:ty, $iotyp:ty,)+})?})*} {}) => {
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

	(@emit_enum {$($env:tt)*} {$($parsed:tt)*} {} {$info:tt} $($t:tt)*) => {
		$crate::ocaml_defs!(@emit_enum {$($env)*} {$($parsed)* {$info}} {} $($t)*);
	};

	(@emit_enum $env:tt {$($parsed:tt)*} $parsing:tt {$info:tt} $($t:tt)*) => {
		$crate::ocaml_defs!(@emit_enum $env {$($parsed)* {$info $parsing}} {} $($t)*);
	};
	// with ocaml_type
	(@emit_enum $env:tt $parsed:tt {$($parsing:tt)*}
		{$info:tt {$iname:ident,$typ:ty,$iotyp:ty} $($other:tt)*} $($t:tt)*) => {
		$crate::ocaml_defs!(
			@emit_enum $env $parsed {$($parsing)* $iname, $typ, $iotyp,}
			{$info $($other)*} $($t)*
		);
	};

	// without ocaml_type
	(@emit_enum {$($env:tt)*} {$($parsed:tt)*} {$($parsing:tt)*}
		{$info:tt {$iname:ident,$typ:ty} $($other:tt)*} $($t:tt)*) => {
		$crate::ocaml_defs!(
			@emit_enum {$($env)*} {$($parsed)*} {$($parsing)* $iname, $typ, $typ,}
			{$info $($other)*} $($t)*
		);
	};

	// struct
	(
		$(#[ocaml_type=$otyp:ident])?
		$(#[doc=$doc:expr])*
		$vis:vis struct $name:ident {
			$(
				$(#[ocaml_type=$iotyp:ty])?
				$(#[doc=$idoc:expr])*
				$ivis:vis $item:ident : $typ:ty
			),+ $(,)?
		} $($t:tt)*
	) => {
		$crate::ocaml_defs!(
			@emit_struct {{$($otyp)?} {$($doc)*} {$vis} $name} {}
			$({$ivis $item $($idoc)*} {$typ $(,$iotyp)?})+
		);
		$crate::ocaml_defs!($($t)*);
	};

	(@emit_struct {{$($otyp:ident)?} {$($doc:expr)*} {$vis:vis} $name:ident}
	 {$({$ivis:vis $item:ident $($idoc:expr)*} {$typ:ty, $iotyp:ty})*}) => {
		$(#[doc = $doc])*
		$vis struct $name {
			$(
				$(#[doc = $idoc])*
				$ivis $item : $typ
			),+
		}
		$crate::ocaml_interop::impl_conv_ocaml_record! {
			$name $(=> $otyp)? {
				$(
					$item : $iotyp,
				)+
			}
		}
	};

	(@emit_struct $info:tt {$($parsing:tt)*} $iinfo:tt {$typ:ty} $($t:tt)*) => {
		$crate::ocaml_defs!(@emit_struct $info {$($parsing)* $iinfo {$typ, $typ}} $($t)*);
	};

	(@emit_struct $info:tt {$($parsing:tt)*} $iinfo:tt $typs:tt $($t:tt)*) => {
		$crate::ocaml_defs!(@emit_struct $info {$($parsing)* $iinfo $typs} $($t)*);
	};

	// type
	(
		$(#[derive($($derive:ident),*)])?
		$(#[doc=$doc:expr])*
		$vis:vis type $name:ident; $($t:tt)*
	) => {
		$(#[derive($($derive),*)])?
		$(#[doc = $doc])*
		$vis struct $name($crate::ocaml_interop::RawOCaml);
		unsafe impl $crate::ocaml_interop::ToOCaml<$name> for $name {
			fn to_ocaml<'a>(&self, cr: &'a mut $crate::ocaml_interop::OCamlRuntime)
				-> $crate::ocaml_interop::OCaml<'a, $name> {
				unsafe {
					$crate::ocaml_interop::OCaml::new(
						cr,
						self.0
					)
				}
			}
		}
		unsafe impl $crate::ocaml_interop::FromOCaml<$name> for $name {
			fn from_ocaml<'a>(v: $crate::ocaml_interop::OCaml<$name>)
				-> $name {
				unsafe {
					Self(v.raw())
				}
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
	use ocaml_interop::OCamlInt;

	ocaml_defs! {
		#[ocaml_name = "Main.show_error_category"]
		/// hello
		pub fn show_error_category(cat: ErrorCategory) -> String;

		#[ocaml_type = Line]
		pub enum Line {
			NormalLine(#[ocaml_type = String] s: String),
			DisplayLine(s: String),
			NormalLineOption(o: Option<String>, s: String),
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

		pub struct MyStruct {
			a: String,
			#[ocaml_type = OCamlInt]
			b: i64
		}

		type Apple;
	}
}

pub mod range {
	use ocaml_interop::OCamlInt;
	ocaml_defs! {
		pub enum Range {
			Dummy(s: String),
			Normal(
				s: String,
				#[ocaml_type = OCamlInt] i: i64,
				#[ocaml_type = OCamlInt] j: i64,
				#[ocaml_type = OCamlInt] k: i64,
				#[ocaml_type = OCamlInt] l: i64,
			),
		}

		#[ocaml_name = "Range.dummy"]
		pub fn dummy(s: String) -> Range;
		#[ocaml_name = "Range.is_dummy"]
		pub fn is_dummy(r: Range) -> bool;
		#[ocaml_name = "Range.message"]
		pub fn message(r: Range) -> String;
		#[ocaml_name = "Range.to_string"]
		pub fn to_string(r: Range) -> String;
		#[ocaml_name = "Range.get_last"]
		pub fn get_last(r: Range) ->
			#[ocaml_type = Option<(String, OCamlInt, OCamlInt)>]
			Option<(String, i64, i64)>;
		#[ocaml_name = "Range.unite"]
		pub fn unite(r1: Range, r2: Range) -> Range;
		#[ocaml_name = "Range.make"]
		pub fn make(
			s: String,
			#[ocaml_type = OCamlInt] i: i64,
			#[ocaml_type = OCamlInt] j: i64,
			#[ocaml_type = OCamlInt] k: i64
		) -> Range;
		#[ocaml_name = "Range.make_large"]
		pub fn make_large(
			s: String,
			#[ocaml_type = OCamlInt] i: i64,
			#[ocaml_type = OCamlInt] j: i64,
			#[ocaml_type = OCamlInt] k: i64,
			#[ocaml_type = OCamlInt] l: i64
		) -> Range;
	}

	#[test]
	fn test() {
		eprintln!("hello");
	}
}

pub mod store_id {
	use ocaml_interop::OCamlInt;
	ocaml_defs! {
		#[derive(Copy, Clone)]
		pub type StoreID;

		#[ocaml_name = "StoreID.initialize"]
		pub fn initialize(v: ());

		#[ocaml_name = "StoreID.equal"]
		pub fn equal(v1: StoreID, v2: StoreID) -> bool;

		#[ocaml_name = "StoreID.compare"]
		pub fn compare(v1: StoreID, v2: StoreID) -> #[ocaml_type = OCamlInt] i64;

		#[ocaml_name = "StoreID.hash"]
		pub fn hash(v: StoreID) -> #[ocaml_type = OCamlInt] i64;

		#[ocaml_name = "StoreID.fresh"]
		pub fn fresh(v: ()) -> StoreID;

		#[ocaml_name = "StoreID.show_direct"]
		pub fn show_direct(v: StoreID) -> String;
	}

	#[test]
	fn test() {
		initialize(());
		let id0 = fresh(());
		let id1 = fresh(());
		assert!(equal(id1, id1));
		assert!(compare(id0, id1) < 0);
		assert_eq!(&show_direct(id0), "<SID:0>");
		let _ = hash(id0);
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
