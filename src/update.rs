// TODO:
// - Compression
// - Faster MD5Sum
// - Think about how this interacts with systemd (i.e. will systemd try to restart the old binary)

extern crate hex;
extern crate md5;
extern crate nix;
extern crate reqwest;
extern crate std;

use crate::result;

pub const VERSION: Option<&'static str> = option_env!("TTDASH_VERSION");

const TRACK: &'static str = "arm";  // TODO(mrjones): Make this configurable

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TTDashVersion {
    major: i32,
    minor: i32,
}

impl std::fmt::Display for TTDashVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        return write!(f, "{}.{}", self.major, self.minor);
    }
}

impl TTDashVersion {
    pub fn to_string(&self) -> String {
        return format!("{}.{}", self.major, self.minor);
    }
}

#[derive(Serialize, Deserialize)]
pub struct TTDashUpgradeTarget {
    pub version: TTDashVersion,
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
    let body = reqwest::blocking::get(
        &format!("http://linode.mrjon.es/ttdash-{}.version", TRACK))?.text()?;

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
            debug!("LOCAL VERSION: {:?}", local_version);
            debug!("AVAILABLE VERSION: {:?}", available_target.version);
            debug!("TRACK: {}", TRACK);
            if available_target.version > local_version {
                return Some(available_target);
            } else {
                return None;
            }
        },
        (_, Err(remote_err)) => {
            error!("Error determining remote version: {:?}", remote_err);
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

    debug!("md5sum {}", filename);
    let mut disk_file = std::fs::File::open(&filename)?;
    let mut disk_contents = vec![];
    disk_file.read_to_end(&mut disk_contents)?;
    debug!("md5sum read done len={}", disk_contents.len());
    let mut hasher = md5::Md5::new();
    debug!("md5sum hasher newed");
    hasher.update(&disk_contents);
    debug!("md5sum inputted");
    let result = hasher.finalize();
    let encoded_result: String = hex::encode(result.as_slice());
    info!("md5sum: {}", encoded_result);
    return Ok(encoded_result);
}

pub fn upgrade_to(target: &TTDashUpgradeTarget, argv0: &str, argv: &Vec<String>) -> result::TTDashResult<()> {

    let filename = format!("/tmp/ttdash-download-{}.{}",
                           target.version.major, target.version.minor);

    let good_binary_exists =  match md5sum(&filename) {
        Ok(sum) => (sum == target.md5sum),
        _ => false,
    };

    if !good_binary_exists {
        let mut local_file = std::fs::File::create(&filename)?;
        info!("Downloading {}...", &target.url);
        reqwest::blocking::get(&target.url)?.copy_to(&mut local_file)?;
        info!("Download complete.");
    }

    assert_eq!(target.md5sum, md5sum(&filename)?);

    info!("Downloaded version {}.", target.version);

    std::fs::set_permissions(
        &filename, std::os::unix::fs::PermissionsExt::from_mode(0o777))?;
    std::fs::remove_file("/tmp/ttdash.prev").ok();
    std::fs::copy(argv0, "/tmp/ttdash.prev")?;
    std::fs::rename(&filename, argv0)?;

    let argv0_c = std::ffi::CString::new(argv0).expect("cstringing argv0");
    let argv_c: Vec<std::ffi::CString> = argv.iter()
        .map(|s| std::ffi::CString::new(s.as_str()).expect("cstringing argv")).collect();

    info!("Execing new binary.");
    nix::unistd::execv(&argv0_c, argv_c.as_slice()).unwrap();

    return Ok(());
}
