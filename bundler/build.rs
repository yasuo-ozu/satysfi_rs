use std::{env, process::Command};

fn main() {
	let out_dir = env::var("OUT_DIR").unwrap();
	let switch_name = format!("{}/", out_dir);
	let satysfi_dir = "../ocaml/satysfi";
	Command::new("rm")
		.args(&[
			"-rf",
			&format!("{}/libsatysfi.a", out_dir),
			&format!("{}/libsatysfi.o", out_dir),
			&format!("{}/_opam", out_dir),
		])
		.status()
		.expect("rm failed");
	Command::new("opam")
		.args(&["switch", "create", &switch_name, "--no-install"])
		.status()
		.expect("Cannot create switch");
	Command::new("opam")
		.args(&["install", "--switch", &switch_name, "dune"])
		.status()
		.expect("Cannot install dune");
	Command::new("opam")
		.args(&[
			"repository",
			"--switch",
			&switch_name,
			"add",
			"satysfi-external",
			"https://github.com/gfngfn/satysfi-external-repo.git",
		])
		.status()
		.expect("Cannot register satysfi-external repo");
	Command::new("opam")
		.args(&[
			"repository",
			"--switch",
			&switch_name,
			"add",
			"satyrographos-repo",
			"https://github.com/na4zagin3/satyrographos-repo.git",
		])
		.status()
		.expect("Cannot register satyrographos-repo");
	Command::new("opam")
		.args(&["update", "--switch", &switch_name])
		.status()
		.expect("Cannot run opam update");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&["pin", "--switch", &switch_name, ".", "-y"])
		.status()
		.expect("Cannot run opam pin");
	Command::new("opam")
		.current_dir(satysfi_dir)
		.args(&[
			"exec",
			"--switch",
			&switch_name,
			"--",
			"dune",
			"build",
			&format!("--build-dir={}/_build", out_dir),
			"src/main.a",
		])
		.status()
		.expect("Cannot run dune build");
	Command::new("cp")
		.args(&[
			&format!("{}/_build/default/src/main.a", out_dir),
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
