// TODOs:
// - Signatures / checksums
extern crate reqwest;
extern crate std;

use result;

pub const VERSION: Option<&'static str> = option_env!("TTDASH_VERSION");

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
struct TTDashVersion {
    major: i32,
    minor: i32,
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

fn available_version() -> result::TTDashResult<TTDashVersion> {
    return parse_version(&reqwest::get("http://linode.mrjon.es/ttdash.version")?.text()?);
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

pub fn binary_update_available() -> Option<String> {
    match (local_version(), available_version()) {
        (Ok(local_version), Ok(available_version)) => {
            println!("LOCAL VERSION: {:?}", local_version);
            println!("AVAILABLE VERSION: {:?}", available_version);
            if available_version > local_version {
                return Some(format!("{}.{}", available_version.major, available_version.minor));
            } else {
                return None;
            }
        },
        _ => {
            return None;
        }
    };
}

pub fn upgrade_to(version: &str) -> result::TTDashResult<()> {
    let filename = format!("/tmp/ttdash-download-{}", version);

    let mut buffer = std::fs::File::create(&filename)?;

    reqwest::get(&format!(
        "http://linode.mrjon.es/ttdash-{}", version))?
        .copy_to(&mut buffer)?;

    return Ok(());
}
