use flate2::read::GzDecoder;
use reqwest::blocking::{Client, ClientBuilder, Response};
use reqwest::header::{ACCEPT, CONTENT_TYPE, WWW_AUTHENTICATE};
use reqwest::{Method, Proxy};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use tar::{Builder as TarBuilder, Header as TarHeader};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
struct Cli {
    image: String,
    output_dir: PathBuf,
    file_name: Option<String>,
    tag: Option<String>,
    platform: String,
    platform_os: Option<String>,
    platform_arch: Option<String>,
    username: Option<String>,
    password: Option<String>,
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    no_proxy: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub no_proxy: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegistryAuth {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug)]
pub struct PullImageRequest {
    pub repository: String,
    pub tag: Option<String>,
    pub output_dir: PathBuf,
    pub tar_file_name: Option<String>,
    pub platform_os: String,
    pub platform_architecture: String,
    pub auth: RegistryAuth,
    pub proxy: Option<ProxyConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PullImageResult {
    image_ref: String,
    tar_path: PathBuf,
    layer_count: usize,
}

#[derive(Debug, Deserialize)]
struct ManifestList {
    manifests: Vec<ManifestListEntry>,
}

#[derive(Debug, Deserialize)]
struct ManifestListEntry {
    digest: String,
    platform: Option<ManifestPlatform>,
}

#[derive(Debug, Deserialize)]
struct ManifestPlatform {
    architecture: Option<String>,
    os: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageManifest {
    config: ManifestDescriptor,
    layers: Vec<ManifestDescriptor>,
}

#[derive(Debug, Deserialize)]
struct ManifestDescriptor {
    #[serde(rename = "mediaType")]
    media_type: Option<String>,
    digest: String,
}

#[derive(Debug)]
struct AuthChallenge {
    realm: String,
    service: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Clone)]
struct LayerDownloadJob {
    digest: String,
    media_type: Option<String>,
    expected_diff_digest: String,
    blob_tar_path: String,
    temp_blob_path: PathBuf,
}

pub fn run_cli() -> Result<(), String> {
    let cli = Cli::parse(env::args().skip(1))?;
    let (platform_os, platform_architecture) = parse_platform(&cli)?;
    let proxy = build_proxy_config(&cli);
    let request = PullImageRequest {
        repository: cli.image,
        tag: cli.tag,
        output_dir: cli.output_dir,
        tar_file_name: cli.file_name,
        platform_os,
        platform_architecture,
        auth: RegistryAuth {
            username: cli.username,
            password: cli.password,
        },
        proxy,
    };

    let result = pull_image_as_tar(request)?;
    println!("Image: {}", result.image_ref);
    println!("Layers: {}", result.layer_count);
    println!("Tar: {}", result.tar_path.display());
    Ok(())
}

pub fn print_cli_result(result: Result<(), String>) {
    if let Err(err) = result {
        if err == "__HELP__" {
            print_help();
        } else if err == "__VERSION__" {
            println!("dockerhub-image-tar {VERSION}");
        } else {
            eprintln!("error: {err}");
            eprintln!("run `dockerhub-image-tar --help` for usage");
            std::process::exit(1);
        }
    }
}

impl Cli {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut cli = Cli {
            image: String::new(),
            output_dir: PathBuf::from("."),
            file_name: None,
            tag: None,
            platform: "linux/amd64".to_string(),
            platform_os: None,
            platform_arch: None,
            username: None,
            password: None,
            http_proxy: None,
            https_proxy: None,
            no_proxy: None,
        };
        let mut args = args.into_iter().peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => return Err("__HELP__".to_string()),
                "-V" | "--version" => return Err("__VERSION__".to_string()),
                "-o" | "--output-dir" => {
                    cli.output_dir = PathBuf::from(next_value(&mut args, &arg)?)
                }
                "-f" | "--file-name" => cli.file_name = Some(next_value(&mut args, &arg)?),
                "--tag" => cli.tag = Some(next_value(&mut args, &arg)?),
                "--platform" => cli.platform = next_value(&mut args, &arg)?,
                "--platform-os" => cli.platform_os = Some(next_value(&mut args, &arg)?),
                "--platform-arch" => cli.platform_arch = Some(next_value(&mut args, &arg)?),
                "-u" | "--username" => cli.username = Some(next_value(&mut args, &arg)?),
                "-p" | "--password" => cli.password = Some(next_value(&mut args, &arg)?),
                "--http-proxy" => cli.http_proxy = Some(next_value(&mut args, &arg)?),
                "--https-proxy" => cli.https_proxy = Some(next_value(&mut args, &arg)?),
                "--no-proxy" => cli.no_proxy = Some(next_value(&mut args, &arg)?),
                value if value.starts_with('-') => return Err(format!("unknown option: {value}")),
                value => {
                    if !cli.image.is_empty() {
                        return Err(format!("unexpected extra argument: {value}"));
                    }
                    cli.image = value.to_string();
                }
            }
        }
        if cli.image.is_empty() {
            return Err("missing required image argument".to_string());
        }
        Ok(cli)
    }
}

fn next_value<I>(args: &mut std::iter::Peekable<I>, option: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a value"))
}

fn print_help() {
    println!(
        "dockerhub-image-tar {VERSION}

Pull a Docker Hub image and export a Docker-loadable tar archive.

Usage:
  dockerhub-image-tar [OPTIONS] <IMAGE>

Arguments:
  <IMAGE>  Docker Hub image, for example nginx:latest, library/redis:7, or my-org/app:1.0

Options:
  -o, --output-dir <DIR>        Output directory [default: .]
  -f, --file-name <NAME>        Output tar file name
      --tag <TAG>               Override the image tag
      --platform <OS/ARCH>      Target platform [default: linux/amd64]
      --platform-os <OS>        Target platform OS
      --platform-arch <ARCH>    Target platform architecture
  -u, --username <USERNAME>     Docker Hub username
  -p, --password <PASSWORD>     Docker Hub password or access token
      --http-proxy <URL>        HTTP proxy URL
      --https-proxy <URL>       HTTPS proxy URL
      --no-proxy <LIST>         Comma-separated proxy bypass list
  -h, --help                    Print help
  -V, --version                 Print version"
    );
}

fn parse_platform(cli: &Cli) -> Result<(String, String), String> {
    let (platform_os, platform_architecture) = cli
        .platform
        .split_once('/')
        .map(|(os, arch)| (os.trim().to_string(), arch.trim().to_string()))
        .unwrap_or_else(|| ("linux".to_string(), "amd64".to_string()));

    let platform_os = cli
        .platform_os
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or(platform_os);
    let platform_architecture = cli
        .platform_arch
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or(platform_architecture);

    if platform_os.is_empty() || platform_architecture.is_empty() {
        return Err(
            "platform must include both OS and architecture, for example linux/amd64".to_string(),
        );
    }
    Ok((platform_os, platform_architecture))
}

fn build_proxy_config(cli: &Cli) -> Option<ProxyConfig> {
    let proxy = ProxyConfig {
        http_proxy: cli.http_proxy.clone(),
        https_proxy: cli.https_proxy.clone(),
        no_proxy: cli.no_proxy.clone(),
    };
    if clean_opt(proxy.http_proxy.as_deref()).is_none()
        && clean_opt(proxy.https_proxy.as_deref()).is_none()
        && clean_opt(proxy.no_proxy.as_deref()).is_none()
    {
        None
    } else {
        Some(proxy)
    }
}

pub fn pull_image_as_tar(request: PullImageRequest) -> Result<PullImageResult, String> {
    let api_registry = "registry-1.docker.io";
    let (repo, tag) =
        resolve_repository_and_tag(request.repository.trim(), request.tag.as_deref())?;
    let image_ref = format!("{repo}:{tag}");

    fs::create_dir_all(&request.output_dir)
        .map_err(|e| format!("failed to create output directory: {e}"))?;
    let tar_file_name = request
        .tar_file_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}-{}.tar", sanitize_file_name(&repo), tag));
    let tar_path = request.output_dir.join(tar_file_name);

    let client = build_http_client(request.proxy.as_ref(), api_registry)?;
    let manifest_accept = [
        "application/vnd.docker.distribution.manifest.list.v2+json",
        "application/vnd.docker.distribution.manifest.v2+json",
        "application/vnd.oci.image.index.v1+json",
        "application/vnd.oci.image.manifest.v1+json",
    ]
    .join(", ");
    let mut token: Option<String> = None;
    let base = format!("https://{api_registry}");

    let mut manifest_response = request_registry(
        &client,
        Method::GET,
        &format!("{base}/v2/{repo}/manifests/{tag}"),
        Some(&manifest_accept),
        &request.auth,
        &mut token,
    )?;
    let content_type = manifest_response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let manifest_bytes = read_response_bytes(&mut manifest_response)?;
    let selected_manifest = if content_type.contains("manifest.list.v2+json")
        || content_type.contains("image.index.v1+json")
    {
        let manifest_list: ManifestList = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| format!("failed to parse image index: {e}"))?;
        let selected_entry = select_manifest_entry(
            &manifest_list,
            Some(&request.platform_os),
            Some(&request.platform_architecture),
        )
        .ok_or_else(|| "image index does not contain a usable manifest".to_string())?;
        let mut response = request_registry(
            &client,
            Method::GET,
            &format!("{base}/v2/{repo}/manifests/{}", selected_entry.digest),
            Some(&manifest_accept),
            &request.auth,
            &mut token,
        )?;
        let bytes = read_response_bytes(&mut response)?;
        serde_json::from_slice::<ImageManifest>(&bytes)
            .map_err(|e| format!("failed to parse image manifest: {e}"))?
    } else {
        serde_json::from_slice::<ImageManifest>(&manifest_bytes)
            .map_err(|e| format!("failed to parse image manifest: {e}"))?
    };

    let config_blob_path = digest_to_blob_tar_path(&selected_manifest.config.digest)?;
    let config_blob = download_blob_bytes(
        &client,
        &base,
        &repo,
        &selected_manifest.config.digest,
        &request.auth,
        &mut token,
    )?;
    let config_json: Value = serde_json::from_slice(&config_blob)
        .map_err(|e| format!("failed to parse image config: {e}"))?;
    let layer_diff_ids = resolve_layer_diff_ids(&config_json, selected_manifest.layers.len())?;
    if layer_diff_ids.is_empty() {
        return Err("image has no exportable filesystem layers".to_string());
    }

    let mut layer_jobs = Vec::with_capacity(selected_manifest.layers.len());
    for (index, layer) in selected_manifest.layers.iter().enumerate() {
        let expected_diff_digest = layer_diff_ids[index].clone();
        let layer_digest_key = digest_to_key(&expected_diff_digest)?;
        layer_jobs.push(LayerDownloadJob {
            digest: layer.digest.clone(),
            media_type: layer.media_type.clone(),
            expected_diff_digest: expected_diff_digest.clone(),
            blob_tar_path: digest_to_blob_tar_path(&expected_diff_digest)?,
            temp_blob_path: request
                .output_dir
                .join(format!(".tmp-layer-{index}-{layer_digest_key}.blob")),
        });
    }

    download_layers_in_parallel(
        request.proxy.as_ref(),
        api_registry,
        &base,
        &repo,
        &request.auth,
        &layer_jobs,
    )?;

    let tar_file =
        File::create(&tar_path).map_err(|e| format!("failed to create tar file: {e}"))?;
    let mut tar_builder = TarBuilder::new(tar_file);

    append_bytes_entry(&mut tar_builder, &config_blob_path, &config_blob)?;

    let layer_descriptors = layer_jobs
        .iter()
        .map(|layer| {
            let layer_size = fs::metadata(&layer.temp_blob_path)
                .map_err(|e| format!("failed to read temporary layer size: {e}"))?
                .len();
            Ok(json!({
                "mediaType": "application/vnd.oci.image.layer.v1.tar",
                "digest": layer.expected_diff_digest,
                "size": layer_size
            }))
        })
        .collect::<Result<Vec<Value>, String>>()?;

    let manifest_payload = json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": selected_manifest.config.digest,
            "size": config_blob.len()
        },
        "layers": layer_descriptors
    });
    let manifest_bytes = serde_json::to_vec(&manifest_payload)
        .map_err(|e| format!("failed to serialize manifest: {e}"))?;
    let manifest_digest = build_sha256_digest(&manifest_bytes);
    let manifest_blob_path = digest_to_blob_tar_path(&manifest_digest)?;
    append_bytes_entry(&mut tar_builder, &manifest_blob_path, &manifest_bytes)?;

    let layer_paths = layer_jobs
        .iter()
        .map(|layer| layer.blob_tar_path.clone())
        .collect::<Vec<String>>();
    let mut layer_sources = serde_json::Map::new();
    for layer in &layer_jobs {
        let layer_size = fs::metadata(&layer.temp_blob_path)
            .map_err(|e| format!("failed to read temporary layer size: {e}"))?
            .len();
        layer_sources.insert(
            layer.expected_diff_digest.clone(),
            json!({
                "mediaType": "application/vnd.oci.image.layer.v1.tar",
                "size": layer_size,
                "digest": layer.expected_diff_digest
            }),
        );
    }
    append_json_entry(
        &mut tar_builder,
        "manifest.json",
        &json!([{
            "Config": config_blob_path,
            "RepoTags": [image_ref],
            "Layers": layer_paths,
            "LayerSources": layer_sources
        }]),
    )?;

    let last_layer_key = digest_to_key(&layer_diff_ids[layer_diff_ids.len() - 1])?;
    let mut tag_map = serde_json::Map::new();
    tag_map.insert(tag.clone(), Value::String(last_layer_key));
    let mut repositories_map = serde_json::Map::new();
    repositories_map.insert(repo.clone(), Value::Object(tag_map));
    append_json_entry(
        &mut tar_builder,
        "repositories",
        &Value::Object(repositories_map),
    )?;

    let compat_entries =
        build_compat_layer_jsons(&selected_manifest, &layer_diff_ids, &config_json);
    for compat in compat_entries {
        let compat_digest = build_sha256_digest(&compat);
        let compat_blob_path = digest_to_blob_tar_path(&compat_digest)?;
        append_bytes_entry(&mut tar_builder, &compat_blob_path, &compat)?;
    }

    for layer in &layer_jobs {
        append_file_entry(
            &mut tar_builder,
            &layer.blob_tar_path,
            &layer.temp_blob_path,
        )?;
        fs::remove_file(&layer.temp_blob_path)
            .map_err(|e| format!("failed to remove temporary layer: {e}"))?;
    }

    append_json_entry(
        &mut tar_builder,
        "oci-layout",
        &json!({
            "imageLayoutVersion": "1.0.0"
        }),
    )?;
    append_json_entry(
        &mut tar_builder,
        "index.json",
        &json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.oci.image.index.v1+json",
            "manifests": [{
                "mediaType": "application/vnd.oci.image.manifest.v1+json",
                "digest": manifest_digest,
                "size": manifest_bytes.len(),
                "annotations": {
                    "io.containerd.image.name": image_ref,
                    "org.opencontainers.image.ref.name": tag
                }
            }]
        }),
    )?;

    tar_builder
        .finish()
        .map_err(|e| format!("failed to finish tar archive: {e}"))?;

    Ok(PullImageResult {
        image_ref,
        tar_path,
        layer_count: selected_manifest.layers.len(),
    })
}

fn build_http_client(proxy: Option<&ProxyConfig>, target_host: &str) -> Result<Client, String> {
    let mut builder = ClientBuilder::new().user_agent("dockerhub-image-tar/0.1");
    if let Some(cfg) = proxy {
        let bypass = should_bypass_proxy(target_host, cfg.no_proxy.as_deref());
        if !bypass {
            if let Some(http_proxy) = clean_opt(cfg.http_proxy.as_deref()) {
                builder = builder.proxy(Proxy::http(http_proxy).map_err(format_reqwest_err)?);
            }
            if let Some(https_proxy) = clean_opt(cfg.https_proxy.as_deref()) {
                builder = builder.proxy(Proxy::https(https_proxy).map_err(format_reqwest_err)?);
            }
        }
    }
    builder.build().map_err(format_reqwest_err)
}

fn request_registry(
    client: &Client,
    method: Method,
    url: &str,
    accept: Option<&str>,
    auth: &RegistryAuth,
    token: &mut Option<String>,
) -> Result<Response, String> {
    let method_name = method.as_str().to_string();
    let mut req = client.request(method.clone(), url);
    if let Some(accept_value) = accept {
        req = req.header(ACCEPT, accept_value);
    }
    if let Some(existing_token) = token.clone() {
        req = req.bearer_auth(existing_token);
    } else if let (Some(username), Some(password)) = (
        clean_opt(auth.username.as_deref()),
        clean_opt(auth.password.as_deref()),
    ) {
        req = req.basic_auth(username, Some(password));
    }

    let response = req.send().map_err(format_reqwest_err)?;
    if response.status() != reqwest::StatusCode::UNAUTHORIZED {
        return ensure_success(response)
            .map_err(|err| format!("{err}; request: {} {}", method_name, url));
    }

    let challenge = response
        .headers()
        .get(WWW_AUTHENTICATE)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_auth_challenge);

    if let Some(challenge) = challenge {
        let fetched_token = fetch_bearer_token(client, &challenge, auth)?;
        *token = Some(fetched_token.clone());
        let mut retry = client.request(method, url).bearer_auth(fetched_token);
        if let Some(accept_value) = accept {
            retry = retry.header(ACCEPT, accept_value);
        }
        let retry_response = retry.send().map_err(format_reqwest_err)?;
        return ensure_success(retry_response)
            .map_err(|err| format!("{err}; request: {} {}", method_name, url));
    }

    Err("registry authentication failed; check credentials and repository permissions".to_string())
}

fn fetch_bearer_token(
    client: &Client,
    challenge: &AuthChallenge,
    auth: &RegistryAuth,
) -> Result<String, String> {
    let mut request = client.get(&challenge.realm);
    if let Some(service) = clean_opt(challenge.service.as_deref()) {
        request = request.query(&[("service", service)]);
    }
    if let Some(scope) = clean_opt(challenge.scope.as_deref()) {
        request = request.query(&[("scope", scope)]);
    }
    if let (Some(username), Some(password)) = (
        clean_opt(auth.username.as_deref()),
        clean_opt(auth.password.as_deref()),
    ) {
        request = request.basic_auth(username, Some(password));
    }

    let response = request.send().map_err(format_reqwest_err)?;
    if !response.status().is_success() {
        return Err(format!(
            "failed to get registry access token: {}",
            response.status()
        ));
    }
    let body: Value = response.json().map_err(format_reqwest_err)?;
    if let Some(token) = body.get("token").and_then(Value::as_str) {
        return Ok(token.to_string());
    }
    if let Some(token) = body.get("access_token").and_then(Value::as_str) {
        return Ok(token.to_string());
    }
    Err("token response did not contain token or access_token".to_string())
}

fn parse_auth_challenge(raw_header: &str) -> Option<AuthChallenge> {
    let header = raw_header.trim();
    if !header.to_ascii_lowercase().starts_with("bearer ") {
        return None;
    }
    let fields = &header[7..];
    let mut params = HashMap::new();
    for part in fields.split(',') {
        let mut pair = part.trim().splitn(2, '=');
        let key = pair.next()?.trim().to_ascii_lowercase();
        let value = pair.next()?.trim().trim_matches('"').to_string();
        params.insert(key, value);
    }
    Some(AuthChallenge {
        realm: params.get("realm")?.to_string(),
        service: params.get("service").cloned(),
        scope: params.get("scope").cloned(),
    })
}

fn download_blob_bytes(
    client: &Client,
    base: &str,
    repo: &str,
    digest: &str,
    auth: &RegistryAuth,
    token: &mut Option<String>,
) -> Result<Vec<u8>, String> {
    let mut response = request_registry(
        client,
        Method::GET,
        &format!("{base}/v2/{repo}/blobs/{digest}"),
        None,
        auth,
        token,
    )?;
    read_response_bytes(&mut response)
}

fn download_blob_to_file(
    client: &Client,
    base: &str,
    repo: &str,
    digest: &str,
    media_type: Option<&str>,
    expected_diff_digest: &str,
    auth: &RegistryAuth,
    token: &mut Option<String>,
    destination: &Path,
) -> Result<(), String> {
    let mut response = request_registry(
        client,
        Method::GET,
        &format!("{base}/v2/{repo}/blobs/{digest}"),
        None,
        auth,
        token,
    )?;
    let mut output =
        File::create(destination).map_err(|e| format!("failed to create temporary layer: {e}"))?;

    let expected_key = digest_to_key(expected_diff_digest)?;
    let source_key = digest_to_key(digest)?;
    let decode_gzip = media_type
        .map(|value| value.contains("gzip"))
        .unwrap_or(false)
        || expected_key != source_key;

    if decode_gzip {
        let mut decoder = GzDecoder::new(response);
        io::copy(&mut decoder, &mut output)
            .map_err(|e| format!("failed to decompress image layer: {e}"))?;
    } else {
        response
            .copy_to(&mut output)
            .map_err(|e| format!("failed to write image layer: {e}"))?;
    }

    let actual_digest = build_sha256_digest_from_file(destination)?;
    if actual_digest != expected_diff_digest {
        return Err(format!(
            "layer digest mismatch, expected {expected_diff_digest}, got {actual_digest}"
        ));
    }
    Ok(())
}

fn download_layers_in_parallel(
    proxy: Option<&ProxyConfig>,
    api_registry: &str,
    base: &str,
    repo: &str,
    auth: &RegistryAuth,
    layer_jobs: &[LayerDownloadJob],
) -> Result<(), String> {
    if layer_jobs.is_empty() {
        return Ok(());
    }

    let client = build_http_client(proxy, api_registry)?;
    let max_parallel = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(4)
        .clamp(2, 8);
    let parallel = std::cmp::min(max_parallel, layer_jobs.len());

    for chunk in layer_jobs.chunks(parallel) {
        let mut handles = Vec::with_capacity(chunk.len());
        for layer in chunk {
            let client = client.clone();
            let base = base.to_string();
            let repo = repo.to_string();
            let auth = auth.clone();
            let digest = layer.digest.clone();
            let media_type = layer.media_type.clone();
            let expected_diff_digest = layer.expected_diff_digest.clone();
            let destination = layer.temp_blob_path.clone();
            handles.push(std::thread::spawn(move || {
                let mut token: Option<String> = None;
                download_blob_to_file(
                    &client,
                    &base,
                    &repo,
                    &digest,
                    media_type.as_deref(),
                    &expected_diff_digest,
                    &auth,
                    &mut token,
                    &destination,
                )
            }));
        }

        for handle in handles {
            let result = handle
                .join()
                .map_err(|_| "image layer download thread panicked".to_string())?;
            result?;
        }
    }

    Ok(())
}

fn select_manifest_entry<'a>(
    manifest_list: &'a ManifestList,
    preferred_os: Option<&str>,
    preferred_architecture: Option<&str>,
) -> Option<&'a ManifestListEntry> {
    let normalized_os = preferred_os
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("linux")
        .to_ascii_lowercase();
    let normalized_arch = preferred_architecture
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("amd64")
        .to_ascii_lowercase();

    let preferred = manifest_list.manifests.iter().find(|entry| {
        let platform = match &entry.platform {
            Some(platform) => platform,
            None => return false,
        };
        let os_ok = platform
            .os
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case(&normalized_os))
            .unwrap_or(false);
        let arch_ok = platform
            .architecture
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case(&normalized_arch))
            .unwrap_or(false);
        os_ok && arch_ok
    });
    preferred.or_else(|| manifest_list.manifests.first())
}

fn append_json_entry(
    tar_builder: &mut TarBuilder<File>,
    entry_path: &str,
    value: &Value,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| format!("failed to serialize JSON: {e}"))?;
    append_bytes_entry(tar_builder, entry_path, &bytes)
}

fn append_bytes_entry(
    tar_builder: &mut TarBuilder<File>,
    entry_path: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let mut header = TarHeader::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar_builder
        .append_data(&mut header, entry_path, Cursor::new(bytes))
        .map_err(|e| format!("failed to write tar entry {entry_path}: {e}"))
}

fn append_file_entry(
    tar_builder: &mut TarBuilder<File>,
    entry_path: &str,
    file_path: &Path,
) -> Result<(), String> {
    tar_builder
        .append_path_with_name(file_path, entry_path)
        .map_err(|e| format!("failed to write tar file entry {entry_path}: {e}"))
}

fn read_response_bytes(response: &mut Response) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    response
        .read_to_end(&mut bytes)
        .map_err(|e| format!("failed to read response: {e}"))?;
    Ok(bytes)
}

fn ensure_success(response: Response) -> Result<Response, String> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let hint = if status == reqwest::StatusCode::NOT_FOUND {
            " (repository path or tag was not found)"
        } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            " (Docker Hub rate limit reached; try authenticating)"
        } else if status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
            " (registry or proxy service unavailable)"
        } else {
            ""
        };
        Err(format!(
            "registry request failed with status {status}{hint}"
        ))
    }
}

fn resolve_repository_and_tag(
    repository: &str,
    request_tag: Option<&str>,
) -> Result<(String, String), String> {
    if repository.is_empty() {
        return Err("image name cannot be empty".to_string());
    }

    let mut normalized_repo = strip_dockerhub_prefix(repository).to_string();
    if let Some((manifest_repo, manifest_tag)) = parse_manifest_style_repo(&normalized_repo) {
        normalized_repo = manifest_repo;
        let tag = pick_effective_tag(request_tag, Some(manifest_tag.as_str()));
        let repo = normalize_dockerhub_repository(normalized_repo.trim())?;
        return Ok((repo, tag));
    }

    let (repo_without_tag, parsed_tag) = split_repo_and_tag(&normalized_repo);
    let tag = pick_effective_tag(request_tag, parsed_tag.as_deref());
    let repo = normalize_dockerhub_repository(repo_without_tag.trim())?;
    Ok((repo, tag))
}

fn pick_effective_tag(request_tag: Option<&str>, parsed_tag: Option<&str>) -> String {
    let explicit = request_tag.map(str::trim).filter(|value| !value.is_empty());
    let use_parsed_tag =
        parsed_tag.is_some() && explicit.map(|value| value == "latest").unwrap_or(true);
    if use_parsed_tag {
        return parsed_tag.unwrap_or("latest").to_string();
    }
    explicit.unwrap_or("latest").to_string()
}

fn strip_dockerhub_prefix(repo_input: &str) -> &str {
    let mut value = repo_input.trim();
    if let Some(without_scheme) = value.strip_prefix("https://") {
        value = without_scheme;
    } else if let Some(without_scheme) = value.strip_prefix("http://") {
        value = without_scheme;
    }
    for prefix in ["docker.io/", "index.docker.io/", "registry-1.docker.io/"] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            return stripped;
        }
    }
    value
}

fn parse_manifest_style_repo(repository: &str) -> Option<(String, String)> {
    let marker = "/manifest/";
    let (repo, tag) = repository.split_once(marker)?;
    let tag = tag.trim();
    if repo.trim().is_empty() || tag.is_empty() {
        return None;
    }
    Some((repo.to_string(), tag.to_string()))
}

fn split_repo_and_tag(repository: &str) -> (String, Option<String>) {
    if repository.contains("@sha256:") {
        return (repository.to_string(), None);
    }
    if let Some((repo, candidate_tag)) = repository.rsplit_once(':') {
        if !candidate_tag.contains('/')
            && !repo.trim().is_empty()
            && !candidate_tag.trim().is_empty()
        {
            return (repo.to_string(), Some(candidate_tag.to_string()));
        }
    }
    (repository.to_string(), None)
}

fn normalize_dockerhub_repository(repository: &str) -> Result<String, String> {
    if repository.is_empty() {
        return Err("image name cannot be empty".to_string());
    }
    if !repository.contains('/') {
        return Ok(format!("library/{repository}"));
    }
    Ok(repository.to_string())
}

fn digest_to_key(digest: &str) -> Result<String, String> {
    digest
        .split_once(':')
        .map(|(_, value)| value.to_string())
        .ok_or_else(|| format!("invalid digest format: {digest}"))
}

fn digest_to_blob_tar_path(digest: &str) -> Result<String, String> {
    let (algorithm, encoded) = digest
        .split_once(':')
        .ok_or_else(|| format!("invalid digest format: {digest}"))?;
    Ok(format!("blobs/{algorithm}/{encoded}"))
}

fn build_sha256_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    format!("sha256:{hash:x}")
}

fn build_sha256_digest_from_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("failed to read image layer: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("failed to read image layer: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let hash = hasher.finalize();
    Ok(format!("sha256:{hash:x}"))
}

fn resolve_layer_diff_ids(config_json: &Value, layer_count: usize) -> Result<Vec<String>, String> {
    let rootfs = config_json
        .get("rootfs")
        .ok_or_else(|| "image config is missing rootfs".to_string())?;
    let diff_ids = rootfs
        .get("diff_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "image config is missing rootfs.diff_ids".to_string())?;
    let mut values = diff_ids
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<String>>();
    if values.len() < layer_count {
        return Err(format!(
            "image layer count mismatch: manifest has {}, config has {} diff_ids",
            layer_count,
            values.len()
        ));
    }
    if values.len() > layer_count {
        values = values.split_off(values.len() - layer_count);
    }
    Ok(values)
}

fn build_compat_layer_jsons(
    selected_manifest: &ImageManifest,
    layer_diff_ids: &[String],
    config_json: &Value,
) -> Vec<Vec<u8>> {
    if selected_manifest.layers.is_empty() || layer_diff_ids.is_empty() {
        return Vec::new();
    }

    let os = config_json
        .get("os")
        .and_then(Value::as_str)
        .unwrap_or("linux")
        .to_string();
    let architecture = config_json
        .get("architecture")
        .and_then(Value::as_str)
        .unwrap_or("amd64")
        .to_string();
    let created = config_json
        .get("created")
        .and_then(Value::as_str)
        .unwrap_or("1970-01-01T00:00:00Z")
        .to_string();

    let mut entries = Vec::with_capacity(layer_diff_ids.len());
    let mut parent_id: Option<String> = None;
    for (index, diff_digest) in layer_diff_ids.iter().enumerate() {
        let id = digest_to_key(diff_digest).unwrap_or_else(|_| sanitize_file_name(diff_digest));
        let mut obj = serde_json::Map::new();
        obj.insert("id".to_string(), Value::String(id.clone()));
        if let Some(parent) = parent_id.clone() {
            obj.insert("parent".to_string(), Value::String(parent));
        }
        obj.insert("created".to_string(), Value::String(created.clone()));
        obj.insert(
            "container_config".to_string(),
            build_empty_container_config(),
        );
        obj.insert("os".to_string(), Value::String(os.clone()));

        if index + 1 == layer_diff_ids.len() {
            obj.insert(
                "architecture".to_string(),
                Value::String(architecture.clone()),
            );
            if let Some(config_value) = config_json.get("config") {
                obj.insert("config".to_string(), config_value.clone());
            }
            if let Some(container_config_value) = config_json.get("container_config") {
                obj.insert(
                    "container_config".to_string(),
                    container_config_value.clone(),
                );
            }
        }

        if let Ok(bytes) = serde_json::to_vec(&Value::Object(obj)) {
            entries.push(bytes);
        }
        parent_id = Some(id);
    }
    entries
}

fn build_empty_container_config() -> Value {
    json!({
        "Hostname": "",
        "Domainname": "",
        "User": "",
        "AttachStdin": false,
        "AttachStdout": false,
        "AttachStderr": false,
        "Tty": false,
        "OpenStdin": false,
        "StdinOnce": false,
        "Env": Value::Null,
        "Cmd": Value::Null,
        "Image": "",
        "Volumes": Value::Null,
        "WorkingDir": "",
        "Entrypoint": Value::Null,
        "OnBuild": Value::Null,
        "Labels": Value::Null
    })
}

fn sanitize_file_name(input: &str) -> String {
    input.replace('/', "_").replace(':', "_")
}

fn should_bypass_proxy(target_host: &str, no_proxy: Option<&str>) -> bool {
    let Some(no_proxy) = clean_opt(no_proxy) else {
        return false;
    };
    no_proxy.split(',').any(|rule| {
        let item = rule.trim();
        if item.is_empty() {
            return false;
        }
        if item == "*" {
            return true;
        }
        if target_host == item {
            return true;
        }
        if let Some(stripped) = item.strip_prefix('.') {
            return target_host.ends_with(stripped);
        }
        target_host.ends_with(&format!(".{item}"))
    })
}

fn clean_opt(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn format_reqwest_err(err: reqwest::Error) -> String {
    if err.is_timeout() {
        return format!("request timed out: {err}");
    }
    if err.is_connect() {
        return format!("connection failed: {err}");
    }
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_args() {
        let cli = Cli::parse(["nginx:1.25".to_string()]).expect("args should parse");
        assert_eq!(cli.image, "nginx:1.25");
        assert_eq!(cli.output_dir, PathBuf::from("."));
        assert_eq!(cli.platform, "linux/amd64");
    }

    #[test]
    fn resolves_official_image_to_library_namespace() {
        let (repo, tag) =
            resolve_repository_and_tag("nginx:1.25", None).expect("image should resolve");
        assert_eq!(repo, "library/nginx");
        assert_eq!(tag, "1.25");
    }

    #[test]
    fn strips_dockerhub_registry_prefix() {
        let (repo, tag) = resolve_repository_and_tag("docker.io/library/redis:7", None)
            .expect("image should resolve");
        assert_eq!(repo, "library/redis");
        assert_eq!(tag, "7");
    }

    #[test]
    fn explicit_tag_overrides_parsed_latest() {
        let (repo, tag) = resolve_repository_and_tag("library/redis:latest", Some("7"))
            .expect("image should resolve");
        assert_eq!(repo, "library/redis");
        assert_eq!(tag, "7");
    }
}
