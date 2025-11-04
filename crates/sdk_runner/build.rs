use flate2::read::GzDecoder;
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;

const TSGO_VERSION: &str = "2025-11-04";
const TSGO_BASE_URL: &str = "https://github.com/sxzz/tsgo-releases/releases/download";

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    // Use a project-local bin directory instead of OUT_DIR
    // This persists across builds and can be checked into git or cached
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let bin_dir = manifest_dir.join(".bin");
    fs::create_dir_all(&bin_dir)?;

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // Determine the platform-specific package name
    let platform = match (target_os.as_str(), target_arch.as_str()) {
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", "x86_64") => "darwin-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("linux", "x86_64") => "linux-x64",
        ("windows", "aarch64") => "win32-arm64",
        ("windows", "x86_64") => "win32-x64",
        _ => {
            eprintln!("Warning: Unsupported platform {target_os}-{target_arch}");
            eprintln!("TypeScript type checking will be limited to syntax checking only.");
            return Ok(());
        }
    };

    let binary_name = if target_os == "windows" {
        "tsgo.exe"
    } else {
        "tsgo"
    };
    let binary_path = bin_dir.join(binary_name);

    // Create a version marker file to track which version is installed
    let version_marker = bin_dir.join(".tsgo_version");

    // Skip download if binary exists with correct version
    if binary_path.exists()
        && version_marker.exists()
        && let Ok(installed_version) = fs::read_to_string(&version_marker)
        && installed_version.trim() == TSGO_VERSION
    {
        println!("cargo:rustc-env=TSGO_BINARY_PATH={}", binary_path.display());
        return Ok(());
    }

    // Download URL
    let archive_ext = if target_os == "windows" {
        "zip"
    } else {
        "tar.gz"
    };
    let archive_name = format!("tsgo-{platform}.{archive_ext}");
    let download_url = format!("{TSGO_BASE_URL}/{TSGO_VERSION}/{archive_name}");

    println!("cargo:warning=Downloading typescript-go from: {download_url}");

    // Download the archive
    let response = reqwest::blocking::get(&download_url).expect("Failed to download typescript-go");

    if !response.status().is_success() {
        eprintln!(
            "Failed to download typescript-go: HTTP {}",
            response.status()
        );
        eprintln!("TypeScript type checking will be limited to syntax checking only.");
        return Ok(());
    }

    let archive_bytes = response.bytes().expect("Failed to read download");
    let temp_archive = bin_dir.join(&archive_name);
    fs::write(&temp_archive, &archive_bytes)?;

    println!("cargo:warning=Extracting typescript-go binary...");
    if target_os == "windows" {
        extract_zip(&temp_archive, &bin_dir)?;
    } else {
        extract_tar_gz(&temp_archive, &bin_dir)?;
    }

    let extracted_binary = find_binary(&bin_dir, binary_name)?;
    if extracted_binary != binary_path {
        fs::rename(&extracted_binary, &binary_path)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms)?;
    }

    let _ = fs::remove_file(&temp_archive);

    // Write version marker
    fs::write(&version_marker, TSGO_VERSION)?;

    println!(
        "cargo:warning=TypeScript-Go binary installed at: {}",
        binary_path.display()
    );
    println!("cargo:rustc-env=TSGO_BINARY_PATH={}", binary_path.display());

    Ok(())
}

fn extract_tar_gz(archive_path: &PathBuf, out_dir: &PathBuf) -> io::Result<()> {
    let tar_gz = File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(out_dir)?;
    Ok(())
}

fn extract_zip(archive_path: &PathBuf, out_dir: &Path) -> io::Result<()> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(io::Error::other)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(io::Error::other)?;
        let outpath = out_dir.join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

fn find_binary(dir: &PathBuf, binary_name: &str) -> io::Result<PathBuf> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(binary_name) {
            return Ok(path);
        }

        // Check subdirectories
        if path.is_dir()
            && let Ok(binary) = find_binary(&path, binary_name)
        {
            return Ok(binary);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Binary {binary_name} not found in extracted archive"),
    ))
}
