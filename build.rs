fn main() {
	let ocamlopt = std::env::var("OCAMLOPT").unwrap_or_else(|_| "ocamlopt".to_string());
	let ocaml_path = std::str::from_utf8(
		std::process::Command::new(&ocamlopt)
			.arg("-where")
			.output()
			.unwrap()
			.stdout
			.as_ref(),
	)
	.unwrap()
	.trim()
	.to_owned();
	// println!("cargo:rustc-link-search=/usr/lib/ocaml");
	// println!("cargo:rustc-link-lib=dylib=asmrun_shared");
	// println!("cargo:rustc-link-lib=asmrun"); // caml_startup
	// println!("cargo:rustc-link-lib=unix");
	// println!("cargo:rustc-link-lib=dylib=satysfi");
}
