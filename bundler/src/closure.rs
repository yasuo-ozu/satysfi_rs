use ocaml_interop::{OCaml, OCamlException, OCamlRef, OCamlRuntime, RawOCaml};
use ocaml_sys::{
	caml_callback2_exn, caml_callback3_exn, caml_callbackN_exn, caml_callback_exn,
	extract_exception, is_exception_result,
};

#[derive(Copy, Clone)]
pub struct OCamlClosureExn(*const RawOCaml);

unsafe impl Sync for OCamlClosureExn {}

impl OCamlClosureExn {
	pub fn named(name: &str) -> Option<OCamlClosureExn> {
		let named = unsafe {
			let s = match std::ffi::CString::new(name) {
				Ok(s) => s,
				Err(_) => return None,
			};
			ocaml_sys::caml_named_value(s.as_ptr())
		};
		if named.is_null() || unsafe { ocaml_sys::tag_val(*named) } != ocaml_sys::CLOSURE {
			None
		} else {
			Some(OCamlClosureExn(named))
		}
	}

	pub fn call<'a, T, R>(
		&self,
		cr: &'a mut OCamlRuntime,
		arg: OCamlRef<T>,
	) -> Result<OCaml<'a, R>, OCamlException> {
		let result = unsafe { caml_callback_exn(*self.0, arg.get_raw()) };
		self.handle_call_result(cr, result)
	}

	pub fn call2<'a, T, U, R>(
		&self,
		cr: &'a mut OCamlRuntime,
		arg1: OCamlRef<T>,
		arg2: OCamlRef<U>,
	) -> Result<OCaml<'a, R>, OCamlException> {
		let result = unsafe { caml_callback2_exn(*self.0, arg1.get_raw(), arg2.get_raw()) };
		self.handle_call_result(cr, result)
	}

	pub fn call3<'a, T, U, V, R>(
		&self,
		cr: &'a mut OCamlRuntime,
		arg1: OCamlRef<T>,
		arg2: OCamlRef<U>,
		arg3: OCamlRef<V>,
	) -> Result<OCaml<'a, R>, OCamlException> {
		let result =
			unsafe { caml_callback3_exn(*self.0, arg1.get_raw(), arg2.get_raw(), arg3.get_raw()) };
		self.handle_call_result(cr, result)
	}

	pub fn call_n<'a, R>(
		&self,
		cr: &'a mut OCamlRuntime,
		args: &mut [RawOCaml],
	) -> Result<OCaml<'a, R>, OCamlException> {
		let len = args.len();
		let result = unsafe { caml_callbackN_exn(*self.0, len, args.as_mut_ptr()) };
		self.handle_call_result(cr, result)
	}

	#[inline]
	fn handle_call_result<'a, R>(
		&self,
		cr: &'a mut OCamlRuntime,
		result: RawOCaml,
	) -> Result<OCaml<'a, R>, OCamlException> {
		if is_exception_result(result) {
			unsafe { Err(OCamlException::of(extract_exception(result))) }
		} else {
			unsafe { Ok(OCaml::new(cr, result)) }
		}
	}
}
