use std::env;
use std::process::Command;

fn main() {
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86_64" => "x64",
        "x86" => "x86",
        _ => panic!("unsupported target architecture"),
    };
    let out_dir = env::var("OUT_DIR").unwrap();
    let res_path = format!("{out_dir}\\resource.res");
    let obj_path = format!("{out_dir}\\resource.obj");
    let output = Command::new("cmd")
        .arg("/C")
        .arg(format!(
            "vcvarsall {arch} && rc /fo {res_path} resource.rc && cvtres /MACHINE:{arch} {res_path}"
        ))
        .output()
        .unwrap();
    if output.status.success() {
        println!("cargo:rustc-link-arg={obj_path}");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("{}", stderr);
    }
}
