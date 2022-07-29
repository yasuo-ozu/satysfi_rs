extern crate pkg_config;

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::{env, process::Command};
use std::{ffi, fs, io};

fn generate_glue(srcdir: &Path, output: &Path) -> Result<(), std::io::Error> {
	let ignore_mod = [
		"DirectedGraph.",
		"Display.",
		"HashTree.",
		"StoreID.set",
		"StoreID.reset",
		"StoreID.reset",
		"Typeenv.fresh_type_id",
		"Typeenv.add_constructor",
		"Typeenv.register_type",
		"Main.main",
		"MyUtil.( >>= )",
		// "MyUtil.( += )",
		// "MyUtil.( @|> )",
		"MyUtil.return",
		"MyUtil.err",
		"MyUtil.continue",
		"MyUtil.escape",
		"MyUtil.force",
	];
	fn find_files(dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
		let mut ret = Vec::new();
		for entry in fs::read_dir(dir)? {
			let entry = entry?;
			let path = entry.path();
			if entry.file_type()?.is_dir() {
				ret.extend(find_files(&path)?);
			} else if path.extension() == Some(ffi::OsStr::new("mli")) {
				ret.push(path);
			}
		}
		Ok(ret)
	}
	let mut glue = vec![];
	for fname in find_files(srcdir)? {
		let mut modname = fname.file_stem().unwrap().to_str().unwrap().chars();
		// e.g.  src/testItem.mli -> TestItem
		let modname: String = modname
			.next()
			.unwrap()
			.to_uppercase()
			.chain(modname)
			.collect();
		let f = fs::File::open(fname)?;
		for line in io::BufReader::new(f).lines() {
			if let Some(s) = line?.trim().strip_prefix("val") {
				if let Some(i) = s.find(':') {
					let func = s.get(0..i).unwrap().trim();
					let fullname = format!("{}.{}", &modname, func);
					if !ignore_mod.iter().any(|m| fullname.starts_with(m)) {
						glue.push(format!("    Callback.register \"{0}\" {0} ", &fullname));
					}
				}
			}
		}
	}
	{
		let mut output = fs::File::create(output)?;
		writeln!(
			output,
			"let main_lib () =\n{}",
			glue.iter()
				.map(|s| s.as_ref())
				.collect::<Vec<_>>()
				.join(" ;\n")
		)?;
	}
	Ok(())
}

fn test(cmdname: &str, chdir: Option<&str>, args: &[&str]) -> Result<bool, String> {
	eprintln!("[build.rs] Running {} {:?}", cmdname, args);
	let mut cmd = Command::new(cmdname);
	let cmd = match chdir {
		Some(s) => cmd.current_dir(s),
		None => &mut cmd,
	};
	Ok(cmd
		.args(args)
		.status()
		.map_err(|_| format!("Cannot run {} {:?}", cmdname, args))?
		.success())
}

fn get_output(cmdname: &str, chdir: Option<&str>, args: &[&str]) -> Result<String, String> {
	eprintln!("[build.rs] Running {} {:?}", cmdname, args);
	let mut cmd = Command::new(cmdname);
	let cmd = match chdir {
		Some(s) => cmd.current_dir(s),
		None => &mut cmd,
	};
	let out = cmd
		.args(args)
		.output()
		.map_err(|_| format!("Cannot run {} {:?}", cmdname, args))?;
	if out.status.success() {
		Ok(std::str::from_utf8(out.stdout.as_ref())
			.unwrap()
			.trim()
			.to_owned())
	} else {
		Err(format!(
			"Command error {} {:?}, {}",
			cmdname,
			args,
			std::str::from_utf8(out.stderr.as_ref()).unwrap()
		))
	}
}

fn runcmd(cmdname: &str, chdir: Option<&str>, args: &[&str]) -> Result<(), String> {
	if test(cmdname, chdir, args)? {
		Ok(())
	} else {
		Err(format!("Command error {} {:?}", cmdname, args))
	}
}

fn join(args: &[&str]) -> String {
	args.iter()
		.cloned()
		.collect::<PathBuf>()
		.into_os_string()
		.into_string()
		.unwrap()
}

fn check_static_available(lib: pkg_config::Library, lib_dir: &str) -> bool {
	let system_roots = if cfg!(target_os = "macos") {
		vec![PathBuf::from("/Library"), PathBuf::from("/System")]
	} else {
		let sysroot = env::var("PKG_CONFIG_SYSROOT_DIR")
			.or_else(|_| env::var("SYSROOT"))
			.map(PathBuf::from);

		if cfg!(target_os = "windows") {
			if let Ok(sysroot) = sysroot {
				vec![sysroot]
			} else {
				vec![]
			}
		} else {
			vec![sysroot.unwrap_or_else(|_| PathBuf::from("/usr"))]
		}
	};

	let link_paths = lib
		.link_paths
		.iter()
		.cloned()
		.chain([PathBuf::from(lib_dir)])
		.collect::<Vec<_>>();

	lib.libs.iter().all(|name| {
		let libname = format!("lib{}.a", name);

		link_paths.iter().any(|dir| {
			!system_roots.iter().any(|sys| dir.starts_with(sys)) && dir.join(&libname).exists()
		})
	})
}

fn find_static_lib(name: &str, lib_dir: &str) -> Result<bool, String> {
	match pkg_config::Config::new()
		.cargo_metadata(false)
		.env_metadata(false)
		.probe(name)
	{
		Ok(lib) => Ok(check_static_available(lib, lib_dir)),
		Err(pkg_config::Error::ProbeFailure {
			name: _,
			command: _,
			output: _,
		}) => Ok(false),
		Err(e) => Err(format!("{}", e)),
	}
}

fn fetch_missing_lib(out_dir: &str, name: &str) -> Result<(), String> {
	eprintln!("[build.rs] Fetching missing lib {}...", name);
	if &env::var("CARGO_CFG_TARGET_ENV").unwrap() != "gnu" {
		return Err(format!(
			"Cannot find system library: {}\nFetching library is not supported on this target.",
			name
		));
	}
	let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
	match name {
		"libjpeg" => {
			let arch = match arch.as_str() {
				"x86" => "i386",
				"x86_64" => "amd64",
				arch => {
					return Err(format!(
						"Cannot find system library: {}\nFetching library is not supported on {}.",
						name, arch
					));
				}
			};
			let url = format!(
				"https://sourceforge.net/projects/libjpeg-turbo/files/2.1.3/libjpeg-turbo-official_2.1.3_{}.deb", arch
			);
			runcmd("rm", None, &["-rf", &join(&[out_dir, "libjpeg"])])?;
			runcmd("mkdir", None, &["-p", &join(&[out_dir, "libjpeg"])])?;
			runcmd(
				"wget",
				None,
				&[
					"-O",
					&join(&[out_dir, "libjpeg", "libjpeg-turbo.deb"]),
					&url,
				],
			)?;
			runcmd(
				"ar",
				Some(&join(&[out_dir, "libjpeg"])),
				&["x", "libjpeg-turbo.deb"],
			)?;
			runcmd(
				"tar",
				Some(&join(&[out_dir, "libjpeg"])),
				&["-xzvf", "data.tar.gz"],
			)?;
			runcmd(
				"cp",
				None,
				&[
					&join(&[
						out_dir,
						"libjpeg",
						"opt",
						"libjpeg-turbo",
						"lib64",
						"libjpeg.a",
					]),
					out_dir,
				],
			)?;
			eprintln!("[build.rs] Fetching lib {} done.", name);
		}
		_ => {
			return Err(format!("Bad library name: {}", name));
		}
	}

	Ok(())
}

fn fetch_missing_libs(missing_libs_dir: &str) -> Result<(), String> {
	eprintln!("[build.rs] Searching for missing libs...");
	runcmd("mkdir", None, &["-p", missing_libs_dir])?;
	if !find_static_lib("libjpeg", missing_libs_dir)? {
		fetch_missing_lib(missing_libs_dir, "libjpeg")?;
	}
	eprintln!("[build.rs] Searching for missing libs done.");
	Ok(())
}

struct Defer<F: FnMut()>(F);

impl<F> Drop for Defer<F>
where
	F: FnMut(),
{
	fn drop(&mut self) {
		self.0();
	}
}

fn write_version_rs(fname: &str, satysfi_ver: &str) -> Result<(), std::io::Error> {
	let mut fp = std::fs::File::create(fname)?;
	writeln!(
		fp,
		"pub static SATYSFI_VERSION: &'static str = \"{}\";",
		satysfi_ver,
	)?;
	fp.flush()?;
	Ok(())
}

fn generate_version_rs(fname: &str, satysfi_dir: &str) -> Result<(), String> {
	let satysfi_version = get_output("git", Some(satysfi_dir), &["describe"])?;
	write_version_rs(fname, &satysfi_version).map_err(|e| format!("{:?}", e))
}

fn run() -> Result<(), String> {
	let out_dir = env::var("OUT_DIR").unwrap();
	let out_dir = &out_dir;
	let project_dir = &env::var("CARGO_MANIFEST_DIR").unwrap();
	let switch_name = format!("{}/", out_dir);
	let satysfi_dir = &join(&[project_dir, "..", "ocaml", "satysfi"]);
	runcmd(
		"rm",
		None,
		&[
			"-rf",
			&join(&[out_dir, "libsatysfi.a"]),
			&join(&[out_dir, "libsatysfi.so"]),
			// _build error shoule be cleaned every time to prevent error
			&join(&[out_dir, "_build"]),
			&join(&[out_dir, "dune"]),
			&join(&[out_dir, "satysfi.ml"]),
			&join(&[out_dir, "main.ml"]),
			&join(&[out_dir, "main.mli"]),
			&join(&[out_dir, "lib.ml"]),
		],
	)?;
	fetch_missing_libs(&join(&[out_dir, "lib"]))?;
	generate_glue(
		&[satysfi_dir, "src"].iter().cloned().collect::<PathBuf>(),
		&[out_dir, "lib.ml"].iter().cloned().collect::<PathBuf>(),
	)
	.map_err(|e| format!("{}", e))?;
	runcmd(
		"mv",
		None,
		&[
			&join(&[satysfi_dir, "bin", "dune"]),
			&join(&[satysfi_dir, "bin", "satysfi.ml"]),
			&join(&[out_dir]),
		],
	)?;
	runcmd(
		"cp",
		None,
		&[
			&join(&[satysfi_dir, "src", "frontend", "main.ml"]),
			&join(&[satysfi_dir, "src", "frontend", "main.mli"]),
			&join(&[out_dir]),
		],
	)?;
	let _defer = Defer(|| {
		eprintln!("[build.rs] Running defer...");
		let _ = runcmd(
			"rm",
			None,
			&[
				"-f",
				&join(&[satysfi_dir, "bin", "satysfi.ml"]),
				&join(&[satysfi_dir, "src", "frontend", "main.mli"]),
				&join(&[satysfi_dir, "src", "frontend", "main.ml"]),
				&join(&[satysfi_dir, "bin", "dune"]),
			],
		);
		let _ = runcmd(
			"mv",
			None,
			&[
				&join(&[out_dir, "dune"]),
				&join(&[out_dir, "satysfi.ml"]),
				&join(&[satysfi_dir, "bin"]),
			],
		);
		let _ = runcmd(
			"mv",
			None,
			&[
				&join(&[out_dir, "main.ml"]),
				&join(&[out_dir, "main.mli"]),
				&join(&[satysfi_dir, "src", "frontend"]),
			],
		);
	});
	runcmd(
		"cp",
		None,
		&[
			&join(&[project_dir, "patch", "dune"]),
			&join(&[project_dir, "patch", "satysfi.ml"]),
			&join(&[satysfi_dir, "bin"]),
		],
	)?;
	runcmd(
		"sh",
		None,
		&[
			"-c",
			&format!(
				"cat \"{}\" >> \"{}\"",
				join(&[out_dir, "lib.ml"]),
				join(&[satysfi_dir, "src", "frontend", "main.ml"])
			),
		],
	)?;
	runcmd(
		"sh",
		None,
		&[
			"-c",
			&format!(
				"cat \"{}\" >> \"{}\"",
				join(&[project_dir, "patch", "lib.mli"]),
				join(&[satysfi_dir, "src", "frontend", "main.mli"])
			),
		],
	)?;
	test(
		"opam",
		None,
		&["switch", "create", &switch_name, "--no-install"],
	)?;
	runcmd("opam", None, &["install", "--switch", &switch_name, "dune"])?;
	test(
		"opam",
		None,
		&[
			"repository",
			"--switch",
			&switch_name,
			"add",
			"satysfi-external",
			"https://github.com/gfngfn/satysfi-external-repo.git",
		],
	)?;
	test(
		"opam",
		None,
		&[
			"repository",
			"--switch",
			&switch_name,
			"add",
			"satyrographos-repo",
			"https://github.com/na4zagin3/satyrographos-repo.git",
		],
	)?;
	runcmd("opam", None, &["update", "--switch", &switch_name])?;
	runcmd(
		"opam",
		Some(satysfi_dir),
		&["pin", "--switch", &switch_name, ".", "-y"],
	)?;
	runcmd(
		"opam",
		Some(satysfi_dir),
		&[
			"exec",
			"--switch",
			&switch_name,
			"--",
			"dune",
			"build",
			&format!("--build-dir={}", join(&[out_dir, "_build"])),
			&join(&["bin", "satysfi.exe.o"]),
		],
	)?;
	runcmd(
		"opam",
		Some(satysfi_dir),
		&[
			"exec",
			"--switch",
			&switch_name,
			"--",
			"dune",
			"build",
			&format!("--build-dir={}", join(&[out_dir, "_build"])),
			&join(&["bin", "satysfi.so"]),
		],
	)?;
	runcmd(
		"ar",
		None,
		&[
			"qs",
			&join(&[out_dir, "libsatysfi.a"]),
			&join(&[out_dir, "_build", "default", "bin", "satysfi.exe.o"]),
		],
	)?;
	runcmd(
		"cp",
		None,
		&[
			&join(&[out_dir, "_build", "default", "bin", "satysfi.so"]),
			&join(&[out_dir, "libsatysfi.so"]),
		],
	)?;

	generate_version_rs(&join(&[out_dir, "version.rs"]), satysfi_dir)?;

	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed={}", join(&[project_dir, "patch"]));
	// println!("cargo:rerun-if-changed={}", satysfi_dir);
	println!("cargo:rustc-link-search={}", out_dir);
	println!("cargo:rustc-link-lib=satysfi");

	Ok(())
}

fn main() {
	let _ = run().map_err(|s| {
		panic!("[build.rs] Error: {}", s);
	});
}
