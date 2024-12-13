use std::path::Path;
use reqwest;
use zip;
use std::io::Cursor;
use std::{fs};

fn main() {

    // 如果`dist/`目录发生变化，就重新编译，这样可以避免不必要的变更。
    println!("cargo:rerun-if-changed=dist");

    // 区分不同的目标平台和架构
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    const DUCKDB_URL: &str = "https://github.com/duckdb/duckdb/releases/download/v1.1.1/libduckdb-windows-amd64.zip";

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    const DUCKDB_URL: &str = "https://github.com/duckdb/duckdb/releases/download/v1.1.1/libduckdb-windows-arm64.zip";

    #[cfg(target_os = "windows")]
    {
        let out_dir = std::env::current_dir().unwrap();
        let lib_path = if cfg!(target_arch = "x86_64") {
            Path::new(&out_dir).join("lib/duckdb/windows_x86_64")
        } else if cfg!(target_arch = "aarch64") {
            Path::new(&out_dir).join("lib/duckdb/windows_aarch64")
        } else {
            panic!("不支持的 Windows 架构");
        };

        // 确保目标目录存在
        fs::create_dir_all(&lib_path).unwrap_or_else(|e| {
            panic!("创建目录失败: {:?}, 错误: {}", lib_path, e)
        });

        // 下载并解压DuckDB库
        download_and_extract(DUCKDB_URL, &lib_path).unwrap_or_else(|e| {
            panic!("下载和解压 DuckDB 库失败: {}, 错误: {}", DUCKDB_URL, e)
        });

        println!("cargo:rustc-link-search=native={}", lib_path.display());
        println!("cargo:rustc-link-lib=duckdb"); // 根据需要也可以使用静态链接
    }

    // 允许 Tauri 运行其构建过程
    tauri_build::build();
}


fn download_and_extract(url: &str, lib_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Download ZIP file
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?;

    // Use Cursor to convert Bytes into a readable stream
    let mut cursor = Cursor::new(bytes);

    // Create ZIP file
    let zip_path = lib_path.join("duckdb.zip");
    let mut dest = fs::File::create(&zip_path)?;
    std::io::copy(&mut cursor, &mut dest)?;

    // Extract files
    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(lib_path)?;

    // Remove the ZIP file
    fs::remove_file(zip_path)?;

    // move duckdb.dll to src-tauri/lib/windows_resource
    let duckdb_dll_path = lib_path.join("duckdb.dll");
    let windows_resource_path = Path::new(&lib_path).join("../../windows_resource");
    fs::copy(duckdb_dll_path, windows_resource_path.join("duckdb.dll"))?;

    Ok(())
}
