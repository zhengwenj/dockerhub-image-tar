# DockerHub Image Tar

[English](./README_EN.md) | 中文

DockerHub Image Tar 是一个用于拉取 Docker Hub 镜像并导出为 `docker load` 可导入 tar 包的工具。项目包含 Rust CLI 核心能力和基于 Tauri + Vue + Element Plus 的桌面界面。

它适合在不方便直接使用 Docker daemon 拉取镜像的环境中，把 Docker Hub 镜像保存为离线 tar 文件，再复制到目标机器执行 `docker load -i image.tar`。

## 功能特性

- 通过 Docker Registry HTTP API 拉取 Docker Hub 镜像。
- 导出兼容 `docker load` 的 tar 文件。
- 支持公共 Docker Hub 镜像。
- 支持 Docker Hub 用户名 + 密码或 Access Token 认证拉取私有镜像。
- 支持多架构镜像平台选择，默认 `linux/amd64`。
- 支持 HTTP_PROXY、HTTPS_PROXY 和 NO_PROXY。
- 提供桌面界面，支持搜索、选择 tag、选择架构、保存配置和打开导出目录。
- 提供中文和英文界面切换。
- 保留 CLI 用法，便于脚本化调用。

## 已知限制

- 桌面界面的搜索功能使用 Docker Hub 公共搜索 API，私有仓库通常不会出现在搜索结果中。
- 当前桌面界面的 tag 列表查询也基于 Docker Hub Hub API，私有仓库可能无法列出 tag。
- 私有仓库拉取能力在导出阶段由 Registry API 认证完成。后续可以增加"直接输入仓库 + tag 导出"的模式来改善私有镜像体验。
- 目前只面向 Docker Hub Registry，不是通用 OCI Registry 客户端。

## 技术栈

- Rust 2021
- Tauri 2
- Vue 3
- Element Plus
- Vite

## 开发环境

需要安装：

- Rust
- Node.js
- pnpm 或 npm
- Tauri 2 所需系统依赖

安装前端依赖：

```bash
pnpm install
```

如果使用 npm：

```bash
npm install
```

## 桌面应用用法

开发模式运行：

```bash
pnpm tauri dev
```

或：

```bash
npm run tauri dev
```

界面流程：

1. 输入关键字搜索公共镜像，例如 `nginx`、`redis`。
2. 从搜索结果中选择镜像。
3. 选择 tag、目标 OS 和目标架构。
4. 设置输出目录和导出文件名。
5. 如需代理，填写 `HTTP_PROXY`、`HTTPS_PROXY`、`NO_PROXY`。
6. 如需拉取私有镜像，填写 Docker Hub 用户名和 Access Token 或密码。
7. 点击"导出"，生成 tar 文件。
8. 在目标机器上执行 `docker load -i your-image.tar`。

配置文件默认保存到：

```text
C:\Users\<用户名>\AppData\Roaming\cn.com.mlqk.dockerhub-image-tar\config\dockerhub-image-tar.json
```

当前应用标识：

```text
cn.com.mlqk.dockerhub-image-tar
```

## CLI 用法

安装 CLI：

```bash
cargo install --path .
```

拉取公共镜像：

```bash
dockerhub-image-tar nginx:latest -o ./images
```

指定平台：

```bash
dockerhub-image-tar library/redis:7 -o ./images --platform linux/arm64
```

拉取私有镜像：

```bash
dockerhub-image-tar zhengwenj/bughub-blog:latest -o ./images -u "$DOCKERHUB_USERNAME" -p "$DOCKERHUB_TOKEN"
```

指定代理：

```bash
dockerhub-image-tar nginx:latest -o ./images --https-proxy http://127.0.0.1:10808
```

导入镜像：

```bash
docker load -i ./images/library_nginx-latest.tar
```

## CLI 参数

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

## 构建

构建前端：

```bash
pnpm build
```

构建 Rust CLI：

```bash
cargo build --release
```

构建 Tauri 桌面应用：

```bash
pnpm tauri build
```

当前 `src-tauri/tauri.conf.json` 中 `bundle.active` 为 `false`。如需生成安装包，需要按 Tauri 发布需求调整打包配置。

## 项目结构

```text
.
├── .github/           # CI 工作流和 Issue/PR 模板
├── frontend/          # Vue + Element Plus 桌面界面
├── src/               # Rust CLI 和镜像导出核心逻辑
├── src-tauri/         # Tauri 桌面应用入口和命令桥接
├── dist/              # 前端构建产物
├── package.json       # 前端脚本和依赖
├── Cargo.toml         # Rust CLI crate 配置
└── README.md
```

## 安全建议

- 推荐使用 Docker Hub Access Token，不建议长期保存账号密码。
- 不要把配置文件、Access Token 或包含凭据的日志提交到仓库。
- 如果公开 issue 或日志，请先移除用户名、token、代理地址等敏感信息。

## 更新日志

### 1.0.0

- 初始版本。
- 实现 Docker Hub 镜像拉取和 tar 导出。
- 支持 `docker load` 兼容 tar 结构。
- 支持公共镜像和认证私有镜像拉取。
- 支持平台选择、代理配置和自定义输出文件名。
- 新增 Tauri 桌面界面。
- 新增 Docker Hub 搜索、tag 查询、架构选择和导出流程。
- 新增配置保存与读取。
- 新增中英文界面切换。

## License

MIT License. See [LICENSE](./LICENSE).
