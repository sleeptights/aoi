use flate2::{write::GzEncoder, Compression};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

fn find_rc_exe() -> Option<PathBuf> {
    let roots = [
        PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin"),
        PathBuf::from(r"C:\Program Files\Windows Kits\10\bin"),
    ];
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else { continue };
        let mut vers: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        vers.sort();
        vers.reverse();
        for ver in vers {
            let p = ver.join("x64").join("rc.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn embed_icon(manifest: &Path, out: &Path) {
    let ico = manifest
        .join("..")
        .join("src-tauri")
        .join("icons")
        .join("icon.ico");
    println!("cargo:rerun-if-changed={}", ico.display());
    if !ico.exists() {
        return;
    }
    let Ok(ico_abs) = ico.canonicalize() else {
        return;
    };
    let rc_path = out.join("icon.rc");
    let res_path = out.join("icon.res");
    let ico_rc = ico_abs.display().to_string().replace('\\', "\\\\");
    if fs::write(&rc_path, format!("1 ICON \"{ico_rc}\"\n")).is_err() {
        return;
    }
    let rc_exe = find_rc_exe();
    let Some(rc_exe) = rc_exe else {
        return;
    };
    let ok = Command::new(rc_exe)
        .args([
            "/nologo",
            "/fo",
            &res_path.to_string_lossy(),
            &rc_path.to_string_lossy(),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok && res_path.exists() {
        println!("cargo:rustc-link-arg={}", res_path.display());
    }
}

fn main() -> io::Result<()> {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let exe = manifest.join("..").join("dist").join("aoi.exe");
    println!("cargo:rerun-if-changed={}", exe.display());
    if !exe.exists() {
        panic!("сначала собери плеер: dist\\aoi.exe не найден");
    }

    let raw = fs::read(&exe)?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::best());
    gz.write_all(&raw)?;
    let packed = gz.finish()?;

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out.join("aoi.exe.gz"), packed)?;
    println!("cargo:rustc-env=AOI_UNCOMPRESSED={}", raw.len());
    embed_icon(&manifest, &out);
    Ok(())
}
