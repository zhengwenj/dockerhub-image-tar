# DockerHub Image Tar

[中文](./README.md) | English

DockerHub Image Tar is a tool for pulling Docker Hub images and exporting them as `docker load`-compatible tar archives. The project includes a Rust CLI core and a desktop GUI built with Tauri + Vue + Element Plus.

It is ideal for environments where a Docker daemon is not available for direct image pulls — you can save Docker Hub images as offline tar files, copy them to the target machine, and run `docker load -i image.tar`.

## Features

- Pull Docker Hub images via the Docker Registry HTTP API.
- Export tar files compatible with `docker load`.
- Support for public Docker Hub images.
- Support for private image pulls using Docker Hub username + password or Access Token authentication.
- Multi-architecture platform selection, defaulting to `linux/amd64`.
- HTTP_PROXY, HTTPS_PROXY, and NO_PROXY support.
- Desktop GUI with search, tag selection, architecture selection, config persistence, and an option to open the export directory.
- Chinese and English UI switching.
- CLI usage preserved for scripted workflows.

## Known Limitations

- The desktop GUI search relies on the Docker Hub public search API; private repositories typically do not appear in search results.
- The tag list query in the desktop GUI also uses the Docker Hub API; private repositories may not list their tags.
- Private repository pulls are authenticated at the export stage via the Registry API. A future enhancement could add a "direct repo + tag export" mode to improve the private image experience.
- Currently targets only the Docker Hub Registry — it is not a general-purpose OCI Registry client.

## Tech Stack

- Rust 2021
- Tauri 2
- Vue 3
- Element Plus
- Vite

## Development Setup

Prerequisites:

- Rust
- Node.js
- pnpm or npm
- System dependencies required by Tauri 2

Install frontend dependencies:

```bash
pnpm install
```

Or with npm:

```bash
npm install
```

## Desktop App Usage

Run in development mode:

```bash
pnpm tauri dev
```

Or:

```bash
npm run tauri dev
```

GUI workflow:

1. Enter keywords to search for public images, e.g. `nginx`, `redis`.
2. Select an image from the search results.
3. Choose a tag, target OS, and target architecture.
4. Set the output directory and export file name.
5. If a proxy is needed, fill in `HTTP_PROXY`, `HTTPS_PROXY`, and `NO_PROXY`.
6. If pulling private images, enter your Docker Hub username and Access Token or password.
7. Click "Export" to generate the tar file.
8. On the target machine, run `docker load -i your-image.tar`.

The config file is saved by default to:

```text
C:\Users\<username>\AppData\Roaming\cn.com.mlqk.dockerhub-image-tar\config\dockerhub-image-tar.json
```

Current application identifier:

```text
cn.com.mlqk.dockerhub-image-tar
```

## CLI Usage

Install the CLI:

```bash
cargo install --path .
```

Pull a public image:

```bash
dockerhub-image-tar nginx:latest -o ./images
```

Specify a platform:

```bash
dockerhub-image-tar library/redis:7 -o ./images --platform linux/arm64
```

Pull a private image:

```bash
dockerhub-image-tar zhengwenj/bughub-blog:latest -o ./images -u "$DOCKERHUB_USERNAME" -p "$DOCKERHUB_TOKEN"
```

Specify a proxy:

```bash
dockerhub-image-tar nginx:latest -o ./images --https-proxy http://127.0.0.1:10808
```

Import the image:

```bash
docker load -i ./images/library_nginx-latest.tar
```

## CLI Arguments

```text
Usage: dockerhub-image-tar [OPTIONS] <IMAGE>

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
  -V, --version                 Print version
```

## Building

Build the frontend:

```bash
pnpm build
```

Build the Rust CLI:

```bash
cargo build --release
```

Build the Tauri desktop app:

```bash
pnpm tauri build
```

Currently `bundle.active` is set to `false` in `src-tauri/tauri.conf.json`. To generate installers, adjust the bundle configuration according to Tauri's release requirements.

## Project Structure

```text
.
├── .github/           # CI workflows and Issue/PR templates
├── frontend/          # Vue + Element Plus desktop GUI
├── src/               # Rust CLI and image export core logic
├── src-tauri/         # Tauri desktop app entry point and command bridge
├── dist/              # Frontend build output
├── package.json       # Frontend scripts and dependencies
├── Cargo.toml         # Rust CLI crate configuration
└── README.md
```

## Security Recommendations

- Use Docker Hub Access Tokens instead of storing account passwords long-term.
- Do not commit config files, Access Tokens, or logs containing credentials to the repository.
- If sharing issues or logs publicly, remove usernames, tokens, proxy addresses, and other sensitive information first.

## Changelog

### 1.0.0

- Initial release.
- Docker Hub image pull and tar export.
- `docker load`-compatible tar structure.
- Support for both public and authenticated private image pulls.
- Platform selection, proxy configuration, and custom output file names.
- Tauri desktop GUI.
- Docker Hub search, tag query, architecture selection, and export workflow.
- Config save and load.
- Chinese/English UI switching.

## License

MIT License. See [LICENSE](./LICENSE).
