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
	(@default_to {$d:tt}) => { $d };
	(@default_to {$d:tt} $($t:tt)+) => { $($t)+ };
	(@emit_call $cl:ident $cr:ident $arg1:ident) => { $cl.call($cr, $arg1) };
	(@emit_call $cl:ident $cr:ident $arg1:ident $arg2:ident) => { $cl.call2($cr, $arg1, $arg2) };
	(@emit_call $cl:ident $cr:ident $arg1:ident $arg2:ident $arg3:ident) => { $cl.call3($cr, $arg1, $arg2, $arg3) };
	(@emit_call $cl:ident $cr:ident $($arg:ident)+) => {
		$cl.call_n($cr, &mut [$(unsafe { $arg.get_raw() }),+])
	};
	() => ();

	(@emit_fn
		{$($ocamlname:expr)?}
		{$($doc:expr)?} $vis:vis $name:ident {$($($tpar:ident)+)?} {} {}
		{$($rtyp:ty$(,$rotyp:ty)?)?} $($t:tt)*
	) => {
		$crate::ocaml_defs!(@emit_fn
			{$($ocamlname)?} {$($doc)*} $vis $name {$($($tpar)+)?} {} {{(), a ()}} {$($rtyp$(,$rotyp)?)?}
			$($t)*
		);
	};
	(@emit_fn
		{$($ocamlname:expr)?}
		{$($doc:expr)?} $vis:vis $name:ident {$($($tpar:ident)+)?} {$({$inarg:ident $intyp:ty})*} {$({$val:expr, $arg:ident $typ:ty $(,$otyp: ty)?})+}
		{$($rtyp:ty$(,$rotyp:ty)?)?} $($t:tt)*
	) => {
		$(#[doc=$doc])*
		$vis fn $name<'a$($(,$tpar: 'static)+)?>(
			$($inarg: $intyp),*
		) $(-> $rtyp)?
		where
			$($rtyp: $crate::ocaml_interop::FromOCaml<$crate::ocaml_defs!(@default_to {$rtyp} $($rotyp)?)>,)?
			$($typ: $crate::ocaml_interop::ToOCaml<$crate::ocaml_defs!(@default_to {$typ} $($otyp)?)>,)+
		{
			$crate::ocaml_interop::OCamlRuntime::init_persistent();
			$(
				let cr = unsafe { $crate::ocaml_interop::OCamlRuntime::recover_handle() };
				let $arg = <$typ as $crate::ocaml_interop::ToOCaml<$crate::ocaml_defs!(@default_to {$typ} $($otyp)?)>>::to_ocaml(&$val, cr);
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

	// fn
	(
		$(#[ocaml_name=$ocamlname:expr])?
		$(#[doc=$doc:expr])*
		$vis:vis fn $name:ident$(<$($tpar:ident),+>)?(
			$(
				$(#[ocaml_type=$otyp:ty])?
				$arg:ident: $typ:ty
			),* $(,)?
		) $( -> $(#[ocaml_type=$rotyp:ty])? $rtyp:ty)?; $($t:tt)*
	) => {
		$crate::ocaml_defs!(@emit_fn
			{$($ocamlname)?} {$($doc)*} $vis $name {$($($tpar)+)?} {$({$arg $typ})*}{$({$arg, $arg $typ $(,$otyp)?})*} {$($rtyp$(,$rotyp)?)?}
			$($t)*
		);
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
		$vis:vis type $name:ident $(< $($tpar:ident),+ >)? ; $($t:tt)*
	) => {
		$(#[derive($($derive),*)])?
		$(#[doc = $doc])*
		$vis struct $name$(<$($tpar),+>)?($crate::ocaml_interop::RawOCaml$(,::core::marker::PhantomData<($($tpar,)+)>)?);
		unsafe impl$(<$($tpar),+>)? $crate::ocaml_interop::ToOCaml<Self> for $name$(<$($tpar),+>)? {
			fn to_ocaml<'a>(&self, cr: &'a mut $crate::ocaml_interop::OCamlRuntime)
				-> $crate::ocaml_interop::OCaml<'a, Self> {
				unsafe {
					$crate::ocaml_interop::OCaml::new(
						cr,
						self.0
					)
				}
			}
		}
		unsafe impl$(<$($tpar),+>)? $crate::ocaml_interop::FromOCaml<Self> for $name$(<$($tpar),+>)? {
			fn from_ocaml<'a>(v: $crate::ocaml_interop::OCaml<Self>)
				-> Self {
				unsafe {
					Self(v.raw()$(,$crate::ocaml_defs!(@default_to {$($tpar)?} ::core::marker::PhantomData))?)
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

pub mod alist {
	use ocaml_interop::OCamlList;
	ocaml_defs! {
		#[derive(Copy, Clone)]
		pub type AList<T>;

		#[ocaml_name = "Alist.empty"]
		pub fn empty<T>() -> AList<T>;
		#[ocaml_name = "Alist.extend"]
		pub fn extend<T>(a: AList<T>, b: T) -> AList<T>;
		#[ocaml_name = "Alist.append"]
		pub fn append<T>(
			a: AList<T>,
			#[ocaml_type = OCamlList<T>] v: Vec<T>
		) -> AList<T>;
		#[ocaml_name = "Alist.to_list"]
		pub fn to_list<T>(
			a: AList<T>,
		) -> #[ocaml_type = OCamlList<T>] Vec<T>;
		#[ocaml_name = "Alist.to_list_rev"]
		pub fn to_list_rev<T>(
			a: AList<T>,
		) -> #[ocaml_type = OCamlList<T>] Vec<T>;
		#[ocaml_name = "Alist.of_list"]
		pub fn of_list<T>(
			#[ocaml_type = OCamlList<T>] v: Vec<T>
		) -> AList<T>;
		#[ocaml_name = "Alist.chop_last"]
		pub fn chop_last<T>(
			a: AList<T>,
		) -> Option<(AList<T>, T)>;
		#[ocaml_name = "Alist.cat"]
		pub fn cat<T>(
			a: AList<T>,
			b: AList<T>,
		) -> AList<T>;
	}

	#[test]
	fn test() {
		let a: AList<String> = empty();
		let a = extend(a, "a".to_owned());
		let a = extend(a, "b".to_owned());
		assert_eq!(vec!["a".to_owned(), "b".to_owned()], to_list(a.clone()));
		assert_eq!(vec!["b".to_owned(), "a".to_owned()], to_list_rev(a.clone()));
		let b = of_list(vec!["c".to_owned(), "d".to_owned()]);
		let (c, o) = chop_last(b.clone()).unwrap();
		assert_eq!(o, "d".to_owned());
		let (c, o) = chop_last(c).unwrap();
		assert_eq!(o, "c".to_owned());
		assert!(chop_last(c).is_none());
		let d = cat(a, b);
		assert_eq!(
			to_list(d),
			vec![
				"a".to_owned(),
				"b".to_owned(),
				"c".to_owned(),
				"d".to_owned(),
			]
		)
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
