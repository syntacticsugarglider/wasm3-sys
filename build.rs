use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use bindgen::EnumVariation;

static WASM3_SOURCE: &str = "wasm3/source";
const WHITELIST_REGEX_FUNCTION: &str = "([A-Z]|m3_).*";
const WHITELIST_REGEX_TYPE: &str = "(?:I|c_)?[Mm]3.*";
const WHITELIST_REGEX_VAR: &str = WHITELIST_REGEX_TYPE;
const PRIMITIVES: &[&str] = &[
    "f64", "f32", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8",
];

fn gen_bindings() {
    let out_path = PathBuf::from(&env::var("OUT_DIR").unwrap());

    let mut wrapper = String::new();

    fs::read_dir(WASM3_SOURCE)
        .unwrap_or_else(|_| panic!("failed to read {} directory", WASM3_SOURCE))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("h"))
        .for_each(|path| {
            writeln!(
                &mut wrapper,
                "#include \"{}\"",
                path.file_name().unwrap().to_str().unwrap()
            )
            .unwrap()
        });

    let mut bindings = bindgen::Builder::default()
        .header_contents("wrapper.h", &wrapper)
        .use_core()
        .ctypes_prefix("cty")
        .layout_tests(false)
        .default_enum_style(EnumVariation::ModuleConsts)
        .generate_comments(false)
        .whitelist_function(WHITELIST_REGEX_FUNCTION)
        .whitelist_type(WHITELIST_REGEX_TYPE)
        .whitelist_var(WHITELIST_REGEX_VAR)
        .derive_debug(false);

    for &ty in PRIMITIVES.iter() {
        bindings = bindings.blacklist_type(ty);
    }

    bindings
        .clang_args(&[
            "-Iwasm3/source",
            "-Dd_m3LogOutput=0",
            "-Dd_m3Use32BitSlots=0",
        ])
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Unable to write bindings");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    gen_bindings();

    let mut cfg = cc::Build::new();

    cfg.files(
        fs::read_dir(WASM3_SOURCE)
            .unwrap_or_else(|_| panic!("failed to read {} directory", WASM3_SOURCE))
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|p| p.extension().and_then(OsStr::to_str) == Some("c")),
    );
    cfg.cpp(false)
        .define("d_m3LogOutput", Some("0"))
        .warnings(false)
        .extra_warnings(false)
        .include(WASM3_SOURCE);
    if let Ok(extra_clang_args) = std::env::var("BINDGEN_EXTRA_CLANG_ARGS") {
        if let Some(strings) = shlex::split(&extra_clang_args) {
            strings.iter().for_each(|string| {
                cfg.flag(string);
            })
        } else {
            cfg.flag(&extra_clang_args);
        };
    }
    cfg.define("d_m3Use32BitSlots", Some("0"));
    if let Ok(compiler) = std::env::var("XTENSA_CC") {
        cfg.compiler(&compiler);
    }
    cfg.compile("wasm3");
}
