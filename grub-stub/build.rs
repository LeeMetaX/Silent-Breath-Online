//! Assembles `src/boot.S` with NASM into a static archive that Cargo then
//! links into the kernel binary, and forwards the custom linker script.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let asm_src = crate_dir.join("src/boot.S");
    let asm_obj = out_dir.join("boot.o");
    let lib_path = out_dir.join("libboot.a");
    let linker_script = crate_dir.join("linker.ld");

    println!("cargo:rerun-if-changed=src/boot.S");
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=build.rs");

    let nasm_status = Command::new("nasm")
        .arg("-f")
        .arg("elf64")
        .arg("-o")
        .arg(&asm_obj)
        .arg(&asm_src)
        .status()
        .expect("failed to invoke nasm; install with `apt-get install nasm`");
    assert!(nasm_status.success(), "nasm failed to assemble boot.S");

    let _ = std::fs::remove_file(&lib_path);
    let ar_status = Command::new("ar")
        .arg("crus")
        .arg(&lib_path)
        .arg(&asm_obj)
        .status()
        .expect("failed to invoke ar");
    assert!(ar_status.success(), "ar failed to create libboot.a");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=boot");
    println!("cargo:rustc-link-arg-bin=grub-stub=-T{}", linker_script.display());
    println!("cargo:rustc-link-arg-bin=grub-stub=-nostdlib");
    println!("cargo:rustc-link-arg-bin=grub-stub=-static");
    println!("cargo:rustc-link-arg-bin=grub-stub=--no-dynamic-linker");
    println!("cargo:rustc-link-arg-bin=grub-stub=-zmax-page-size=0x1000");
}
