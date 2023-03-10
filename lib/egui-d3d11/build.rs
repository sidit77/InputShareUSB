use std::path::Path;
use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;

fn main() {
    println!("cargo:rerun-if-changed=src/shader.hlsl");
    let hlsl_file = std::fs::read("src/shader.hlsl").unwrap();
    let mut vs_blob = None;
    let mut ps_blob = None;
    let vs_blob = unsafe {
        D3DCompile(
            hlsl_file.as_ptr() as _,
            hlsl_file.len(),
            windows::s!("shader.hlsl"),
            None,
            None,
            windows::s!("vs_main"),
            windows::s!("vs_5_0"),
            0,
            0,
            &mut vs_blob,
            None,
        ).unwrap();
        vs_blob.unwrap()
    };
    let ps_blob = unsafe {
        D3DCompile(
            hlsl_file.as_ptr() as _,
            hlsl_file.len(),
            windows::s!("shader.hlsl"),
            None,
            None,
            windows::s!("ps_main"),
            windows::s!("ps_5_0"),
            0,
            0,
            &mut ps_blob,
            None,
        ).unwrap();
        ps_blob.unwrap()
    };

    let vs_blob = unsafe {
        std::slice::from_raw_parts(
            vs_blob.GetBufferPointer() as *const u8,
            vs_blob.GetBufferSize(),
        )
    };
    let ps_blob = unsafe {
        std::slice::from_raw_parts(
            ps_blob.GetBufferPointer() as *const u8,
            ps_blob.GetBufferSize(),
        )
    };
    std::fs::write(Path::new(&std::env::var("OUT_DIR").unwrap()).join("shader.vs_blob"), vs_blob).unwrap();
    std::fs::write(Path::new(&std::env::var("OUT_DIR").unwrap()).join("shader.ps_blob"), ps_blob).unwrap();
}