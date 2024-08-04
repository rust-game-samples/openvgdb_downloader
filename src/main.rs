use reqwest::Client;
use serde::Deserialize;
use sqlite::Connection;
use std::fs::File;
use std::io::copy;
use std::path::Path;
use zip::ZipArchive;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

#[derive(Deserialize)]
struct UserDefaults {
    version_key: String,
    update_check_key: String,
    update_interval_key: i64,
}

async fn check_for_updates(client: &Client, user_defaults: &UserDefaults) -> Option<(String, String)> {
    let update_url = "https://api.github.com/repos/OpenVGDB/OpenVGDB/releases?page=1&per_page=1";
    let response = client.get(update_url)
        .header("User-Agent", "OpenVGDBDownloader")
        .send()
        .await
        .ok()?
        .json::<Vec<Release>>()
        .await
        .ok()?;

    let current_version = &user_defaults.version_key;
    let mut next_version = current_version.clone();

    for release in response {
        if release.tag_name != *current_version {
            next_version = release.tag_name.clone();
        }
    }

    if next_version != *current_version {
        let download_url = format!("https://github.com/OpenVGDB/OpenVGDB/releases/download/{}/openvgdb.zip", next_version);
        Some((download_url, next_version))
    } else {
        None
    }
}

async fn download_and_extract(url: &str, version: &str, database_path: &Path) -> std::io::Result<()> {
    let response = reqwest::get(url).await.unwrap();
    let mut file = File::create("openvgdb.zip")?;
    copy(&mut response.bytes().await.unwrap().as_ref(), &mut file)?;

    let mut zip = ZipArchive::new(File::open("openvgdb.zip")?)?;
    let mut extracted_file = zip.by_index(0)?;
    let mut out_file = File::create(database_path)?;
    copy(&mut extracted_file, &mut out_file)?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    let user_defaults = UserDefaults {
        version_key: String::from(""),
        update_check_key: String::from(""),
        update_interval_key: 60 * 60 * 24 * 1,
    };

    let database_path = Path::new("openvgdb.sqlite");

    if !database_path.exists() {
        if let Some((url, version)) = check_for_updates(&client, &user_defaults).await {
            download_and_extract(&url, &version, database_path).await.unwrap();
        }
    }

    let conn = Connection::open(database_path).unwrap();
    println!("Database connection established.");
}
