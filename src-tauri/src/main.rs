#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dockerhub_image_tar::{
    pull_image_as_tar as export_image_tar, ProxyConfig, PullImageRequest, RegistryAuth,
};
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportRequest {
    image: String,
    output_dir: String,
    file_name: Option<String>,
    tag: Option<String>,
    platform_os: String,
    platform_arch: String,
    username: Option<String>,
    password: Option<String>,
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    no_proxy: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequest {
    keyword: String,
    proxy: Option<HubProxyConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagQuery {
    repository: String,
    proxy: Option<HubProxyConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HubProxyConfig {
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    no_proxy: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    output_dir: String,
    file_name: String,
    username: String,
    password: String,
    http_proxy: String,
    https_proxy: String,
    no_proxy: String,
    default_tag: String,
    platform_os: String,
    platform_arch: String,
}

#[derive(Debug, Serialize)]
struct RepositorySearchResult {
    repository: String,
    namespace: String,
    name: String,
    description: String,
    star_count: u64,
    pull_count: u64,
    is_official: bool,
}

#[derive(Debug, Serialize)]
struct TagListResult {
    repository: String,
    tags: Vec<TagResult>,
}

#[derive(Debug, Serialize)]
struct TagResult {
    name: String,
    full_size: u64,
    last_pushed: Option<String>,
    architectures: Vec<ArchitectureOption>,
}

#[derive(Debug, Serialize, Clone)]
struct ArchitectureOption {
    os: String,
    architecture: String,
    variant: Option<String>,
    label: String,
}

#[derive(Debug, Deserialize)]
struct DockerSearchResponse {
    results: Vec<DockerSearchItem>,
}

#[derive(Debug, Deserialize)]
struct DockerSearchItem {
    repo_name: String,
    short_description: Option<String>,
    star_count: u64,
    pull_count: u64,
    is_official: bool,
}

#[derive(Debug, Deserialize)]
struct DockerTagsResponse {
    results: Vec<DockerTagItem>,
}

#[derive(Debug, Deserialize)]
struct DockerTagItem {
    name: String,
    full_size: Option<u64>,
    tag_last_pushed: Option<String>,
    images: Vec<DockerImagePlatform>,
}

#[derive(Debug, Deserialize)]
struct DockerImagePlatform {
    architecture: Option<String>,
    os: Option<String>,
    variant: Option<String>,
}

impl ExportRequest {
    fn into_pull_request(self) -> Result<PullImageRequest, String> {
        let image = clean_required(self.image, "image")?;
        let output_dir = clean_required(self.output_dir, "output_dir")?;
        let platform_os = clean_or_default(self.platform_os, "linux");
        let platform_architecture = clean_or_default(self.platform_arch, "amd64");
        let proxy = build_proxy(self.http_proxy, self.https_proxy, self.no_proxy);

        Ok(PullImageRequest {
            repository: image,
            tag: clean_optional(self.tag),
            output_dir: PathBuf::from(output_dir),
            tar_file_name: clean_optional(self.file_name),
            platform_os,
            platform_architecture,
            auth: RegistryAuth {
                username: clean_optional(self.username),
                password: clean_optional(self.password),
            },
            proxy,
        })
    }
}

#[tauri::command]
async fn pull_image_as_tar(
    request: ExportRequest,
) -> Result<dockerhub_image_tar::PullImageResult, String> {
    let request = request.into_pull_request()?;
    tauri::async_runtime::spawn_blocking(move || export_image_tar(request))
        .await
        .map_err(|err| format!("failed to join export task: {err}"))?
}

#[tauri::command]
async fn search_public_images(
    request: SearchRequest,
) -> Result<Vec<RepositorySearchResult>, String> {
    let keyword = clean_required(request.keyword, "keyword")?;
    let proxy = request.proxy.and_then(normalize_hub_proxy);
    tauri::async_runtime::spawn_blocking(move || {
        search_public_images_blocking(&keyword, proxy.as_ref())
    })
    .await
    .map_err(|err| format!("failed to join search task: {err}"))?
}

#[tauri::command]
async fn list_repository_tags(request: TagQuery) -> Result<TagListResult, String> {
    let repository = clean_required(request.repository, "repository")?;
    let proxy = request.proxy.and_then(normalize_hub_proxy);
    tauri::async_runtime::spawn_blocking(move || {
        list_repository_tags_blocking(&repository, proxy.as_ref())
    })
    .await
    .map_err(|err| format!("failed to join tags task: {err}"))?
}

#[tauri::command]
async fn load_app_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    tauri::async_runtime::spawn_blocking(move || load_app_config_blocking(&app))
        .await
        .map_err(|err| format!("failed to join config load task: {err}"))?
}

#[tauri::command]
async fn save_app_config(app: tauri::AppHandle, config: AppConfig) -> Result<AppConfig, String> {
    tauri::async_runtime::spawn_blocking(move || save_app_config_blocking(&app, config))
        .await
        .map_err(|err| format!("failed to join config save task: {err}"))?
}

fn search_public_images_blocking(
    keyword: &str,
    proxy: Option<&HubProxyConfig>,
) -> Result<Vec<RepositorySearchResult>, String> {
    let client = build_hub_client(proxy, "hub.docker.com")?;
    let response = client
        .get("https://hub.docker.com/v2/search/repositories/")
        .query(&[("query", keyword), ("page_size", "24")])
        .send()
        .map_err(|err| format!("failed to search Docker Hub: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Docker Hub search failed: {err}"))?;

    let payload: DockerSearchResponse = response
        .json()
        .map_err(|err| format!("failed to parse Docker Hub search response: {err}"))?;

    Ok(payload
        .results
        .into_iter()
        .map(|item| {
            let repository = normalize_search_repository(&item.repo_name);
            let (namespace, name) = split_namespace_and_name(&repository);
            RepositorySearchResult {
                repository,
                namespace,
                name,
                description: item.short_description.unwrap_or_default(),
                star_count: item.star_count,
                pull_count: item.pull_count,
                is_official: item.is_official,
            }
        })
        .collect())
}

fn list_repository_tags_blocking(
    repository: &str,
    proxy: Option<&HubProxyConfig>,
) -> Result<TagListResult, String> {
    let client = build_hub_client(proxy, "hub.docker.com")?;
    let (namespace, name) = split_namespace_and_name(repository);
    let url = format!("https://hub.docker.com/v2/namespaces/{namespace}/repositories/{name}/tags");
    let response = client
        .get(url)
        .query(&[("page_size", "30")])
        .send()
        .map_err(|err| format!("failed to query repository tags: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Docker Hub tags query failed: {err}"))?;

    let payload: DockerTagsResponse = response
        .json()
        .map_err(|err| format!("failed to parse tags response: {err}"))?;

    let tags = payload
        .results
        .into_iter()
        .map(|tag| TagResult {
            name: tag.name,
            full_size: tag.full_size.unwrap_or(0),
            last_pushed: tag.tag_last_pushed,
            architectures: build_architectures(tag.images),
        })
        .filter(|tag| !tag.architectures.is_empty())
        .collect();

    Ok(TagListResult {
        repository: repository.to_string(),
        tags,
    })
}

fn build_hub_client(proxy: Option<&HubProxyConfig>, target_host: &str) -> Result<Client, String> {
    let mut builder = ClientBuilder::new().user_agent("dockerhub-image-tar-tauri/0.1");
    if let Some(cfg) = proxy {
        let bypass = should_bypass_proxy(target_host, cfg.no_proxy.as_deref());
        if !bypass {
            if let Some(http_proxy) = clean_ref(cfg.http_proxy.as_deref()) {
                builder = builder.proxy(
                    Proxy::http(http_proxy).map_err(|err| format!("invalid HTTP proxy: {err}"))?,
                );
            }
            if let Some(https_proxy) = clean_ref(cfg.https_proxy.as_deref()) {
                builder = builder.proxy(
                    Proxy::https(https_proxy)
                        .map_err(|err| format!("invalid HTTPS proxy: {err}"))?,
                );
            }
        }
    }
    builder
        .build()
        .map_err(|err| format!("failed to build Docker Hub client: {err}"))
}

fn load_app_config_blocking(app: &tauri::AppHandle) -> Result<AppConfig, String> {
    let path = app_config_path(app)?;
    if !path.exists() {
        return Ok(default_app_config());
    }
    let raw =
        fs::read_to_string(&path).map_err(|err| format!("failed to read config file: {err}"))?;
    let mut config = serde_json::from_str::<AppConfig>(&raw)
        .map_err(|err| format!("failed to parse config file {}: {err}", path.display()))?;
    normalize_app_config(&mut config);
    Ok(config)
}

fn save_app_config_blocking(
    app: &tauri::AppHandle,
    mut config: AppConfig,
) -> Result<AppConfig, String> {
    normalize_app_config(&mut config);
    let path = app_config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create config directory: {err}"))?;
    }
    let raw = serde_json::to_string_pretty(&config)
        .map_err(|err| format!("failed to serialize config: {err}"))?;
    fs::write(&path, raw).map_err(|err| format!("failed to write config file: {err}"))?;
    Ok(config)
}

fn app_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;
    dir.push("config");
    dir.push("dockerhub-image-tar.json");
    Ok(dir)
}

fn default_app_config() -> AppConfig {
    AppConfig {
        output_dir: String::new(),
        file_name: String::new(),
        username: String::new(),
        password: String::new(),
        http_proxy: String::new(),
        https_proxy: String::new(),
        no_proxy: String::new(),
        default_tag: "latest".to_string(),
        platform_os: "linux".to_string(),
        platform_arch: "amd64".to_string(),
    }
}

fn normalize_app_config(config: &mut AppConfig) {
    config.output_dir = config.output_dir.trim().to_string();
    config.file_name = config.file_name.trim().to_string();
    config.username = config.username.trim().to_string();
    config.password = config.password.trim().to_string();
    config.http_proxy = config.http_proxy.trim().to_string();
    config.https_proxy = config.https_proxy.trim().to_string();
    config.no_proxy = config.no_proxy.trim().to_string();
    config.default_tag = config.default_tag.trim().to_string();
    config.platform_os = config.platform_os.trim().to_string();
    config.platform_arch = config.platform_arch.trim().to_string();

    if config.default_tag.is_empty() {
        config.default_tag = "latest".to_string();
    }
    if config.platform_os.is_empty() {
        config.platform_os = "linux".to_string();
    }
    if config.platform_arch.is_empty() {
        config.platform_arch = "amd64".to_string();
    }
}

fn normalize_hub_proxy(proxy: HubProxyConfig) -> Option<HubProxyConfig> {
    let http_proxy = clean_optional(proxy.http_proxy);
    let https_proxy = clean_optional(proxy.https_proxy);
    let no_proxy = clean_optional(proxy.no_proxy);
    if http_proxy.is_none() && https_proxy.is_none() && no_proxy.is_none() {
        None
    } else {
        Some(HubProxyConfig {
            http_proxy,
            https_proxy,
            no_proxy,
        })
    }
}

fn build_architectures(images: Vec<DockerImagePlatform>) -> Vec<ArchitectureOption> {
    let mut items = Vec::new();
    for image in images {
        let Some(os) = clean_optional(image.os) else {
            continue;
        };
        let Some(architecture) = clean_optional(image.architecture) else {
            continue;
        };
        if os == "unknown" || architecture == "unknown" {
            continue;
        }
        let variant = clean_optional(image.variant);
        let label = match variant.as_deref() {
            Some(variant) => format!("{os}/{architecture}/{variant}"),
            None => format!("{os}/{architecture}"),
        };
        if items
            .iter()
            .any(|item: &ArchitectureOption| item.label == label)
        {
            continue;
        }
        items.push(ArchitectureOption {
            os,
            architecture,
            variant,
            label,
        });
    }
    items
}

fn normalize_search_repository(value: &str) -> String {
    if value.contains('/') {
        value.to_string()
    } else {
        format!("library/{value}")
    }
}

fn split_namespace_and_name(repository: &str) -> (String, String) {
    if let Some((namespace, name)) = repository.split_once('/') {
        (namespace.to_string(), name.to_string())
    } else {
        ("library".to_string(), repository.to_string())
    }
}

fn clean_required(value: String, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} cannot be empty"));
    }
    Ok(trimmed.to_string())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn clean_or_default(value: String, default_value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_value.to_string()
    } else {
        trimmed.to_string()
    }
}

fn build_proxy(
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    no_proxy: Option<String>,
) -> Option<ProxyConfig> {
    let http_proxy = clean_optional(http_proxy);
    let https_proxy = clean_optional(https_proxy);
    let no_proxy = clean_optional(no_proxy);

    if http_proxy.is_none() && https_proxy.is_none() && no_proxy.is_none() {
        None
    } else {
        Some(ProxyConfig {
            http_proxy,
            https_proxy,
            no_proxy,
        })
    }
}

fn clean_ref(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|item| !item.is_empty())
}

fn should_bypass_proxy(target_host: &str, no_proxy: Option<&str>) -> bool {
    let Some(no_proxy) = clean_ref(no_proxy) else {
        return false;
    };
    no_proxy.split(',').any(|rule| {
        let item = rule.trim();
        if item.is_empty() {
            return false;
        }
        if item == "*" || item == target_host {
            return true;
        }
        if let Some(stripped) = item.strip_prefix('.') {
            return target_host.ends_with(stripped);
        }
        target_host.ends_with(&format!(".{item}"))
    })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            pull_image_as_tar,
            search_public_images,
            list_repository_tags,
            load_app_config,
            save_app_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
