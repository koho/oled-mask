use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

fn main() {
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86_64" => "x64",
        "x86" => "x86",
        _ => panic!("unsupported target architecture"),
    };
    let output = Command::new("cmd")
        .arg("/C")
        .arg(format!(
            "vcvarsall {arch} && where rc && where cvtres && set INCLUDE"
        ))
        .output()
        .unwrap();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("{}", stderr);
    }
    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut rc_path = None;
    let mut cvtres_path = None;
    for line in stdout.lines() {
        let path = Path::new(line);
        if path.try_exists().is_err() {
            continue;
        }
        if rc_path.is_none() && path.file_name() == Some(OsStr::new("rc.exe")) {
            rc_path = Some(line);
        }
        if cvtres_path.is_none() && path.file_name() == Some(OsStr::new("cvtres.exe")) {
            cvtres_path = Some(line);
        }
        if rc_path.is_some() && cvtres_path.is_some() {
            break;
        }
    }
    let include = stdout.lines().last().unwrap();
    env::set_var("INCLUDE", include.strip_prefix("INCLUDE=").unwrap());
    let out_dir = env::var("OUT_DIR").unwrap();
    let res_path = format!("{out_dir}\\resource.res");
    let obj_path = format!("{out_dir}\\resource.obj");
    if Command::new(rc_path.unwrap())
        .arg("/fo")
        .arg(&res_path)
        .arg("resource.rc")
        .status()
        .unwrap()
        .success()
    {
        Command::new(cvtres_path.unwrap())
            .arg(format!("/MACHINE:{arch}"))
            .arg(&res_path)
            .status()
            .unwrap();
    } else {
        panic!("failed to compile resource.rc");
    }
    println!("cargo:rustc-link-arg={obj_path}");
}
