//! Connect with and upload file to Google Drive using their API

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use google_drive3::{DriveHub, api::File, hyper, hyper_rustls, oauth2};
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, ApplicationSecret};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GDApiConfig {
    installed: InstalledApp,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InstalledApp {
    pub client_id: String,
    pub project_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
}

impl From<InstalledApp> for ApplicationSecret {
    fn from(app: InstalledApp) -> Self {
        Self {
            client_id: app.client_id,
            client_secret: app.client_secret,
            token_uri: app.token_uri,
            auth_uri: app.auth_uri,
            redirect_uris: app.redirect_uris,
            project_id: Some(app.project_id),
            client_email: None,
            auth_provider_x509_cert_url: Some(app.auth_provider_x509_cert_url),
            client_x509_cert_url: None,
        }
    }
}

/// Read and parse GD API credentials from JSON file.
fn get_gdapi_config(secrets_file: Option<String>) -> Result<GDApiConfig> {
    let path: PathBuf = if let Some(f) = secrets_file {
        PathBuf::from(f)
    } else {
        [env!("CARGO_MANIFEST_DIR"), "client_secrets.json"].iter().collect()
    };

    let cfg_content = fs::read_to_string(&path)
        .with_context(|| {
            format!("Failed to read GDApi config file {}", path.display())
        })?;

    let config = serde_json::from_str(&cfg_content)?;

    Ok(config)
}

/// Connect to Google Drive and return hub for accessing it.
pub async fn get_drivehub(
    secrets_file: Option<String>,
) -> Result<DriveHub<HttpsConnector<HttpConnector>>> {
    let config: GDApiConfig = get_gdapi_config(secrets_file)?;
    let secret: ApplicationSecret = config.installed.into();

    let auth = InstalledFlowAuthenticator::builder(
        secret,
        InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("tokencache.json")
    .build()
    .await?;

    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http1()
        .build();

    let client = hyper::Client::builder().build(connector);

    Ok(DriveHub::new(client, auth))
}

/// Upload a single file to Google Drive.
pub async fn upload_file_to_drive(
    hub: &DriveHub<HttpsConnector<HttpConnector>>,
    file_path: &str,
    parent_folder_id: Option<&str>,
) -> Result<()> {
    let path = Path::new(file_path);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid file name")
        .unwrap();

    let file_content = fs::read(file_path)?;

    let mut file_metadata = File {
        name: Some(file_name.to_string()),
        ..Default::default()
    };

    if let Some(folder_id) = parent_folder_id {
        file_metadata.parents = Some(vec![folder_id.to_string()]);
    }

    let result = hub.files()
        .create(file_metadata)
        .upload(
            Cursor::new(file_content),
            "application/octet-stream".parse()?,
        )
        .await?;

    let file_id = result.1.id
        .ok_or("No file id returned")
        .unwrap();
    println!("Uploaded '{}' with ID: {}", file_name, file_id);

    Ok(())
}
