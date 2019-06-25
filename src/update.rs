// TODO:
// - Install new binary
// - Compression
// - Faster MD5Sum

extern crate hex;
extern crate md5;
extern crate nix;
extern crate reqwest;
extern crate std;

use result;

pub const VERSION: Option<&'static str> = option_env!("TTDASH_VERSION");

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TTDashVersion {
    pub major: i32,
    pub minor: i32,
}

#[derive(Serialize, Deserialize)]
pub struct TTDashUpgradeTarget {
    version: TTDashVersion,
    md5sum: String,
    url: String,
}

fn parse_version(version_str: &str) -> result::TTDashResult<TTDashVersion> {
    let parts: Vec<i32> = version_str
        .trim()
        .split(".")
        .filter_map(|s| s.parse::<i32>().ok())
        .collect();

    if parts.len() != 2 {
        return Err(result::make_error(&format!(
            "Invalid version string: {}", version_str)));
    }

    return Ok(TTDashVersion{
        major: parts[0],
        minor: parts[1],
    });
}

fn available_target() -> result::TTDashResult<TTDashUpgradeTarget> {
    let body = reqwest::get("http://linode.mrjon.es/ttdash.version")?.text()?;

    let target_info: TTDashUpgradeTarget = serde_json::from_str(&body)?;

    return Ok(target_info);
}

pub fn local_version() -> result::TTDashResult<TTDashVersion> {
    match VERSION {
        Some(local_version_str) => {
            return parse_version(local_version_str);
        },
        None => {
            return Err(result::make_error("No local version set"));
        }
    };
}

pub fn updater_configured() -> bool {
    return local_version().is_ok();
}

pub fn binary_update_available() -> Option<TTDashUpgradeTarget> {
    match (local_version(), available_target()) {
        (Ok(local_version), Ok(available_target)) => {
            println!("LOCAL VERSION: {:?}", local_version);
            println!("AVAILABLE VERSION: {:?}", available_target.version);
            if available_target.version > local_version {
                return Some(available_target);
            } else {
                return None;
            }
        },
        (_, Err(remote_err)) => {
            println!("Error determining remote version: {:?}", remote_err);
            return None;
        }
        _ => {
            return None;
        }
    };
}

fn md5sum(filename: &str) -> result::TTDashResult<String> {
    use std::io::Read;
    use md5::Digest;

    info!("md5sum {}", filename);
    println!("md5sum {}", filename);
    let mut disk_file = std::fs::File::open(&filename)?;
    let mut disk_contents = vec![];
    disk_file.read_to_end(&mut disk_contents)?;
    println!("md5sum read done len={}", disk_contents.len());
    let mut hasher = md5::Md5::new();
    println!("md5sum hasher newed");
    hasher.input(&disk_contents);
    println!("md5sum inputted");
    let result = hasher.result();
    let encoded_result: String = hex::encode(result.as_slice());
    println!("md5sum: {}", encoded_result);
    return Ok(encoded_result);
}

pub fn upgrade_to(target: &TTDashUpgradeTarget, argv0: &str, argv: &Vec<String>) -> result::TTDashResult<()> {

    let filename = format!("/tmp/ttdash-download-{}.{}",
                           target.version.major, target.version.minor);

    println!("Target md5sum: {}", target.md5sum);
    let good_binary_exists =  match md5sum(&filename) {
        Ok(sum) => (sum == target.md5sum),
        _ => false,
    };

    if !good_binary_exists {
        let mut local_file = std::fs::File::create(&filename)?;
        reqwest::get(&target.url)?.copy_to(&mut local_file)?;
    }

    assert_eq!(target.md5sum, md5sum(&filename)?);

    println!("Download complete");

    std::fs::set_permissions(
        &filename, std::os::unix::fs::PermissionsExt::from_mode(0o777))?;
    std::fs::remove_file("/tmp/ttdash.prev").ok();
    std::fs::copy(argv0, "/tmp/ttdash.prev")?;
    std::fs::rename(&filename, argv0)?;

    let argv0_c = std::ffi::CString::new(argv0).expect("cstringing argv0");
    let argv_c: Vec<std::ffi::CString> = argv.iter()
        .map(|s| std::ffi::CString::new(s.as_str()).expect("cstringing argv")).collect();

    println!("Execing!");
    nix::unistd::execv(&argv0_c, &argv_c).unwrap();

    return Ok(());
}
