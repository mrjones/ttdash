extern crate crypto;
extern crate reqwest;
extern crate std;

use result;

pub const VERSION: Option<&'static str> = option_env!("TTDASH_VERSION");

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
struct TTDashVersion {
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

fn local_version() -> result::TTDashResult<TTDashVersion> {
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
    use crypto::digest::Digest;

    println!("md5sum {}", filename);
    let mut disk_file = std::fs::File::open(&filename)?;
    let mut disk_contents = vec![];
    disk_file.read_to_end(&mut disk_contents)?;
    let mut hasher = crypto::md5::Md5::new();
    hasher.input(&disk_contents);
    let result = hasher.result_str();
    println!("md5sum: {}", result);
    return Ok(result);
}

pub fn upgrade_to(target: &TTDashUpgradeTarget) -> result::TTDashResult<()> {

    let filename = format!("/tmp/ttdash-download-{}.{}",
                           target.version.major, target.version.minor);

    match md5sum(&filename) {
        Ok(sum) => {
            if sum == target.md5sum {
                // Someone already successfully completed a download!
                println!("File already exists");
                return Ok(());
            }
        }
        _ => {},
    }

    {
        let mut local_file = std::fs::File::create(&filename)?;
        reqwest::get(&target.url)?.copy_to(&mut local_file)?;
    }

    assert_eq!(target.md5sum, md5sum(&filename)?);

    println!("Download complete");

    return Ok(());
}
