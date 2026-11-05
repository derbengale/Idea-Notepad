use crate::error::AppError;
use reqwest::blocking::{multipart, Client};
use reqwest::Method;
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

const API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const FILE_FIELDS: &str = "id,name,mimeType,modifiedTime,md5Checksum,size,parents,trashed";

#[derive(Debug, Clone)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub modified_time: Option<String>,
    pub md5: Option<String>,
    pub parents: Vec<String>,
    pub trashed: bool,
}

#[derive(Debug, Clone)]
pub struct DriveChange {
    pub file_id: String,
    pub removed: bool,
    pub file: Option<DriveFile>,
}

#[derive(Debug, Clone)]
pub struct ChangesPage {
    pub changes: Vec<DriveChange>,
    pub next_page_token: Option<String>,
    pub new_start_token: Option<String>,
}

pub struct DriveClient {
    http: Client,
    token: String,
}

impl DriveClient {
    pub fn new(token: String) -> Result<Self, AppError> {
        let http = Client::builder().timeout(Duration::from_secs(60)).build()?;
        Ok(Self { http, token })
    }

    fn request(&self, method: Method, url: &str) -> reqwest::blocking::RequestBuilder {
        self.http.request(method, url).bearer_auth(&self.token)
    }

    fn get_json(&self, url: &str) -> Result<Value, AppError> {
        let response = self.request(Method::GET, url).send()?;
        if response.status() == 410 {
            return Err(AppError::not_found("Drive change token expired"));
        }
        let response = response.error_for_status()?;
        Ok(response.json()?)
    }

    pub fn list_all(&self) -> Result<Vec<DriveFile>, AppError> {
        let mut files = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut url = Url::parse(&format!("{API}/files"))?;
            {
                let mut pairs = url.query_pairs_mut();
                pairs.append_pair("spaces", "appDataFolder");
                pairs.append_pair("pageSize", "1000");
                pairs.append_pair("q", "trashed = false");
                pairs.append_pair("fields", &format!("nextPageToken,files({FILE_FIELDS})"));
                if let Some(token) = &page_token {
                    pairs.append_pair("pageToken", token);
                }
            }
            let value = self.get_json(url.as_str())?;
            if let Some(items) = value.get("files").and_then(Value::as_array) {
                for item in items {
                    if let Some(file) = parse_file(item) {
                        files.push(file);
                    }
                }
            }
            page_token = value
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(|text| text.to_string());
            if page_token.is_none() {
                break;
            }
        }
        Ok(files)
    }

    pub fn find_child(&self, name: &str, parent: &str) -> Result<Option<DriveFile>, AppError> {
        let mut url = Url::parse(&format!("{API}/files"))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("spaces", "appDataFolder");
            pairs.append_pair("pageSize", "10");
            pairs.append_pair(
                "q",
                &format!(
                    "name = '{}' and '{}' in parents and trashed = false",
                    name.replace('\'', "\\'"),
                    parent
                ),
            );
            pairs.append_pair("fields", &format!("files({FILE_FIELDS})"));
        }
        let value = self.get_json(url.as_str())?;
        Ok(value
            .get("files")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(parse_file))
    }

    pub fn create_folder(&self, name: &str, parent: &str) -> Result<DriveFile, AppError> {
        let metadata = json!({
            "name": name,
            "mimeType": FOLDER_MIME,
            "parents": [parent]
        });
        let mut url = Url::parse(&format!("{API}/files"))?;
        url.query_pairs_mut().append_pair("fields", FILE_FIELDS);
        let response = self
            .request(Method::POST, url.as_str())
            .json(&metadata)
            .send()?;
        let response = response.error_for_status()?;
        let value: Value = response.json()?;
        parse_file(&value).ok_or_else(|| AppError::io("Drive did not return the created folder"))
    }

    pub fn ensure_folder(&self, name: &str, parent: &str) -> Result<String, AppError> {
        if let Some(folder) = self.find_child(name, parent)? {
            return Ok(folder.id);
        }
        Ok(self.create_folder(name, parent)?.id)
    }

    pub fn download(&self, file_id: &str) -> Result<Vec<u8>, AppError> {
        let url = format!("{API}/files/{file_id}?alt=media");
        let response = self.request(Method::GET, &url).send()?;
        let response = response.error_for_status()?;
        Ok(response.bytes()?.to_vec())
    }

    pub fn get_metadata(&self, file_id: &str) -> Result<DriveFile, AppError> {
        let mut url = Url::parse(&format!("{API}/files/{file_id}"))?;
        url.query_pairs_mut()
            .append_pair("fields", FILE_FIELDS)
            .append_pair("spaces", "appDataFolder");
        let value = self.get_json(url.as_str())?;
        parse_file(&value).ok_or_else(|| AppError::not_found("Drive file not found"))
    }

    pub fn create_file(
        &self,
        name: &str,
        parent: &str,
        mime_type: &str,
        bytes: &[u8],
        app_properties: Value,
    ) -> Result<DriveFile, AppError> {
        let metadata = json!({
            "name": name,
            "mimeType": mime_type,
            "parents": [parent],
            "appProperties": app_properties
        });
        let part = multipart::Part::bytes(bytes.to_vec())
            .file_name(name.to_string())
            .mime_str(mime_type)
            .map_err(|error| AppError::validation(format!("Invalid MIME type: {error}")))?;
        let form = multipart::Form::new()
            .text("metadata", metadata.to_string())
            .part("file", part);
        let mut url = Url::parse(&format!("{UPLOAD}/files"))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("uploadType", "multipart");
            pairs.append_pair("fields", FILE_FIELDS);
        }
        let response = self
            .request(Method::POST, url.as_str())
            .multipart(form)
            .send()?;
        let response = response.error_for_status()?;
        let value: Value = response.json()?;
        parse_file(&value).ok_or_else(|| AppError::io("Drive did not return the created file"))
    }

    pub fn update_file(
        &self,
        file_id: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<DriveFile, AppError> {
        let mut url = Url::parse(&format!("{UPLOAD}/files/{file_id}"))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("uploadType", "media");
            pairs.append_pair("fields", FILE_FIELDS);
        }
        let response = self
            .request(Method::PATCH, url.as_str())
            .header(reqwest::header::CONTENT_TYPE, mime_type)
            .body(bytes.to_vec())
            .send()?;
        let response = response.error_for_status()?;
        let value: Value = response.json()?;
        parse_file(&value).ok_or_else(|| AppError::io("Drive did not return the updated file"))
    }

    pub fn delete_file(&self, file_id: &str) -> Result<(), AppError> {
        let url = format!("{API}/files/{file_id}");
        let response = self.request(Method::DELETE, &url).send()?;
        if response.status() == 404 {
            return Ok(());
        }
        response.error_for_status()?;
        Ok(())
    }

    pub fn start_page_token(&self) -> Result<String, AppError> {
        let mut url = Url::parse(&format!("{API}/changes/startPageToken"))?;
        url.query_pairs_mut().append_pair("spaces", "appDataFolder");
        let value = self.get_json(url.as_str())?;
        value
            .get("startPageToken")
            .and_then(Value::as_str)
            .map(|token| token.to_string())
            .ok_or_else(|| AppError::network("Drive did not return a start page token"))
    }

    pub fn changes(&self, page_token: &str) -> Result<ChangesPage, AppError> {
        let mut url = Url::parse(&format!("{API}/changes"))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("pageToken", page_token);
            pairs.append_pair("spaces", "appDataFolder");
            pairs.append_pair("pageSize", "1000");
            pairs.append_pair("includeRemoved", "true");
            pairs.append_pair(
                "fields",
                &format!(
                    "nextPageToken,newStartPageToken,changes(fileId,removed,file({FILE_FIELDS}))"
                ),
            );
        }
        let value = self.get_json(url.as_str())?;
        let mut changes = Vec::new();
        if let Some(items) = value.get("changes").and_then(Value::as_array) {
            for item in items {
                let file_id = item
                    .get("fileId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                if file_id.is_empty() {
                    continue;
                }
                let removed = item
                    .get("removed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let file = item.get("file").and_then(parse_file);
                changes.push(DriveChange {
                    file_id,
                    removed,
                    file,
                });
            }
        }
        Ok(ChangesPage {
            changes,
            next_page_token: value
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(|text| text.to_string()),
            new_start_token: value
                .get("newStartPageToken")
                .and_then(Value::as_str)
                .map(|text| text.to_string()),
        })
    }
}

fn parse_file(value: &Value) -> Option<DriveFile> {
    let id = value.get("id")?.as_str()?.to_string();
    Some(DriveFile {
        id,
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        mime_type: value
            .get("mimeType")
            .and_then(Value::as_str)
            .unwrap_or("application/octet-stream")
            .to_string(),
        modified_time: value
            .get("modifiedTime")
            .and_then(Value::as_str)
            .map(|text| text.to_string()),
        md5: value
            .get("md5Checksum")
            .and_then(Value::as_str)
            .map(|text| text.to_string()),
        parents: value
            .get("parents")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|text| text.to_string())
                    .collect()
            })
            .unwrap_or_default(),
        trashed: value
            .get("trashed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

pub fn is_folder(file: &DriveFile) -> bool {
    file.mime_type == FOLDER_MIME
}

pub fn is_markdown(file: &DriveFile) -> bool {
    file.name.ends_with(".md")
}

pub fn is_json(file: &DriveFile) -> bool {
    file.name.ends_with(".json")
}
