use std::{env, process::Command};

fn main() {
	let out_dir = env::var("OUT_DIR").unwrap();
	let satysfi_dir = "./contrib/satysfi";
	Command::new("rm")
		.args(&["-rf", &format!("{}/_opam", satysfi_dir)])
		.status()
		.expect("rm _opam failed");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&["switch", "create", ".", "--no-install"])
		.status()
		.expect("Cannot create switch");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&["install", "dune"])
		.status()
		.expect("Cannot install dune");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&[
			"repository",
			"add",
			"satysfi-external",
			"https://github.com/gfngfn/satysfi-external-repo.git",
		])
		.status()
		.expect("Cannot register satysfi-external repo");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&[
			"repository",
			"add",
			"satyrographos-repo",
			"https://github.com/na4zagin3/satyrographos-repo.git",
		])
		.status()
		.expect("Cannot register satyrographos-repo");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&["update"])
		.status()
		.expect("Cannot run opam update");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&["pin", ".", "-y"])
		.status()
		.expect("Cannot run opam pin");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&["exec", "--", "dune", "build", "src/main.a"])
		.status()
		.expect("Cannot run dune build");
	Command::new("rm")
		.args(&[
			"-f",
			&format!("{}/libsatysfi.a", out_dir),
			&format!("{}/libsatysfi.o", out_dir),
		])
		.status()
		.expect("rm failed");
	Command::new("cp")
		.args(&[
			&format!("{}/_build/default/src/main.a", satysfi_dir),
			&format!("{}/libsatysfi.a", out_dir),
		])
		.status()
		.expect("File copy failed.");
	//Command::new("ar")
	//	.args(&[
	//		"qs",
	//		&format!("{}/libsatysfi.a", out_dir),
	//		&format!("{}/libsatysfi.o", out_dir),
	//	])
	//	.status()
	//	.expect("ar failed");

	println!("cargo:rerun-if-changed={}", satysfi_dir);
	println!("cargo:rustc-link-search={}", out_dir);
	println!("cargo:rustc-link-lib=static=satysfi");
}
