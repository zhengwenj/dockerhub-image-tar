import { createApp, computed, onMounted, reactive, ref } from "vue";
import ElementPlus, { ElMessage } from "element-plus";
import "element-plus/dist/index.css";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir, join } from "@tauri-apps/api/path";
import "./styles.css";

const messages = {
  zh: {
    appSubtitle: "Docker Hub / 公共镜像拉取",
    appTitle: "Docker Hub 工作区",
    language: "语言",
    queryPanel: "查询与拉取参数",
    keyword: "关键字",
    keywordPlaceholder: "输入关键字，如 nginx, redis",
    outputDir: "默认输出目录",
    outputDirPlaceholder: "~/docker-images",
    fileName: "导出文件名",
    fileNamePlaceholder: "留空自动生成",
    selectedImage: "已选镜像",
    selectedImagePlaceholder: "请先从右侧选择镜像",
    selectTag: "选择 Tag",
    selectTagPlaceholder: "请先选择镜像",
    architectureCount: "{count} 个架构",
    targetOs: "目标 OS",
    targetArch: "目标架构",
    proxySection: "Docker Hub 代理",
    authSection: "私有仓库认证",
    username: "Docker Hub 用户名",
    usernamePlaceholder: "私有仓库时填写",
    password: "Access Token / 密码",
    passwordPlaceholder: "推荐 Access Token",
    saveConfig: "保存配置",
    search: "搜索",
    export: "导出",
    results: "搜索结果",
    count: "共 {count} 条",
    emptyResults: "还没有搜索结果",
    imageName: "镜像名",
    description: "描述",
    noDescription: "无描述",
    action: "操作",
    choose: "选择",
    official: "Official",
    stars: "Stars",
    pulls: "Pulls",
    exportSuccess: "导出成功",
    finishedAt: "完成时间：{time}",
    openDirectory: "打开目录",
    image: "镜像",
    file: "文件",
    readyTitle: "就绪",
    readyBody: "输入关键字搜索 Docker Hub 公共镜像。",
    configReadFailedTitle: "配置读取失败",
    configSaved: "配置已保存",
    enterKeyword: "请输入搜索关键字",
    searchingTitle: "正在搜索",
    searchingBody: "正在请求 Docker Hub 公共搜索接口。",
    searchDoneTitle: "搜索完成",
    searchDoneBody: "共返回 {count} 条镜像。",
    searchFailedTitle: "搜索失败",
    loadingImageTitle: "正在加载镜像信息",
    loadingImageBody: "正在查询 {repository} 的 tag 和架构。",
    imageSelectedTitle: "镜像已选择",
    imageSelectedBody: "Tag 与架构已加载，可以导出。",
    noTagsTitle: "没有可用 Tag",
    noTagsBody: "该镜像未返回可导出的 tag 信息。",
    loadingImageFailedTitle: "加载镜像信息失败",
    chooseImageFirst: "请先选择镜像",
    chooseTagArch: "请选择 Tag 和架构",
    fillOutputDir: "请填写输出目录",
    exportingTitle: "正在导出",
    exportingBody: "{image}:{tag} 将按 {platform} 导出。",
    exportDoneTitle: "导出完成",
    exportDoneBody: "镜像 tar 已经生成。",
    exportDoneToast: "镜像导出完成",
    exportFailedTitle: "导出失败",
    openDirFailed: "打开目录失败：{error}",
    browse: "浏览",
  },
  en: {
    appSubtitle: "Docker Hub / Public image export",
    appTitle: "Docker Hub Workspace",
    language: "Language",
    queryPanel: "Search and pull settings",
    keyword: "Keyword",
    keywordPlaceholder: "Search images, e.g. nginx, redis",
    outputDir: "Default output directory",
    outputDirPlaceholder: "~/docker-images",
    fileName: "Export file name",
    fileNamePlaceholder: "Leave empty to auto-generate",
    selectedImage: "Selected image",
    selectedImagePlaceholder: "Select an image from the results",
    selectTag: "Tag",
    selectTagPlaceholder: "Select an image first",
    architectureCount: "{count} architectures",
    targetOs: "Target OS",
    targetArch: "Target arch",
    proxySection: "Docker Hub proxy",
    authSection: "Private repository auth",
    username: "Docker Hub username",
    usernamePlaceholder: "Required for private repositories",
    password: "Access token / password",
    passwordPlaceholder: "Access token recommended",
    saveConfig: "Save settings",
    search: "Search",
    export: "Export",
    results: "Search results",
    count: "{count} total",
    emptyResults: "No search results yet",
    imageName: "Image",
    description: "Description",
    noDescription: "No description",
    action: "Action",
    choose: "Choose",
    official: "Official",
    stars: "Stars",
    pulls: "Pulls",
    exportSuccess: "Export complete",
    finishedAt: "Finished at: {time}",
    openDirectory: "Open folder",
    image: "Image",
    file: "File",
    readyTitle: "Ready",
    readyBody: "Enter a keyword to search Docker Hub public images.",
    configReadFailedTitle: "Failed to load settings",
    configSaved: "Settings saved",
    enterKeyword: "Enter a search keyword",
    searchingTitle: "Searching",
    searchingBody: "Requesting the Docker Hub public search API.",
    searchDoneTitle: "Search complete",
    searchDoneBody: "{count} images returned.",
    searchFailedTitle: "Search failed",
    loadingImageTitle: "Loading image metadata",
    loadingImageBody: "Loading tags and architectures for {repository}.",
    imageSelectedTitle: "Image selected",
    imageSelectedBody: "Tags and architectures are ready for export.",
    noTagsTitle: "No available tags",
    noTagsBody: "This image did not return exportable tag metadata.",
    loadingImageFailedTitle: "Failed to load image metadata",
    chooseImageFirst: "Select an image first",
    chooseTagArch: "Select a tag and architecture",
    fillOutputDir: "Enter an output directory",
    exportingTitle: "Exporting",
    exportingBody: "{image}:{tag} will be exported as {platform}.",
    exportDoneTitle: "Export complete",
    exportDoneBody: "The image tar file has been generated.",
    exportDoneToast: "Image export complete",
    exportFailedTitle: "Export failed",
    openDirFailed: "Failed to open folder: {error}",
    browse: "Browse",
  },
};

const App = {
  setup() {
    const savedLanguage = localStorage.getItem("dockerhub-image-tar-language");
    const language = ref(savedLanguage === "en" ? "en" : "zh");
    const languageOptions = [
      { label: "中文", value: "zh" },
      { label: "English", value: "en" },
    ];
    const keyword = ref("");
    const searching = ref(false);
    const saving = ref(false);
    const exporting = ref(false);
    const loadingTags = ref(false);
    const results = ref([]);
    const selectedRepository = ref(null);
    const tags = ref([]);
    const selectedTagName = ref("");
    const selectedOs = ref("linux");
    const selectedArchLabel = ref("linux/amd64");
    const exportResult = ref(null);
    const status = reactive({
      type: "info",
      titleKey: "readyTitle",
      bodyKey: "readyBody",
      titleParams: {},
      bodyParams: {},
      bodyText: "",
    });
    const config = reactive({
      outputDir: "",
      fileName: "",
      username: "",
      password: "",
      httpProxy: "",
      httpsProxy: "",
      noProxy: "",
    });

    const t = (key, params = {}) => {
      const template = messages[language.value]?.[key] ?? messages.zh[key] ?? key;
      return Object.entries(params).reduce(
        (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
        template,
      );
    };
    const statusTitle = computed(() => t(status.titleKey, status.titleParams));
    const statusBody = computed(() => status.bodyText || t(status.bodyKey, status.bodyParams));
    const selectedTag = computed(() => tags.value.find((item) => item.name === selectedTagName.value) || null);
    const osOptions = computed(() => {
      const items = selectedTag.value?.architectures || [];
      return [...new Set(items.map((item) => item.os))];
    });
    const archOptions = computed(() => {
      const items = selectedTag.value?.architectures || [];
      return items.filter((item) => item.os === selectedOs.value);
    });
    const selectedArchitecture = computed(
      () => archOptions.value.find((item) => item.label === selectedArchLabel.value) || archOptions.value[0] || null,
    );

    function setLanguage(value) {
      language.value = value;
      localStorage.setItem("dockerhub-image-tar-language", value);
    }

    function setStatus(type, titleKey, bodyKey, bodyParams = {}, bodyText = "") {
      status.type = type;
      status.titleKey = titleKey;
      status.bodyKey = bodyKey;
      status.titleParams = {};
      status.bodyParams = bodyParams;
      status.bodyText = bodyText;
    }

    function clean(value) {
      const trimmed = String(value ?? "").trim();
      return trimmed.length ? trimmed : null;
    }

    function normalizeSearchKeyword(value) {
      const text = clean(value);
      if (!text) return null;
      if (text.includes("@")) return text.split("@")[0] || text;
      const lastSlash = text.lastIndexOf("/");
      const lastColon = text.lastIndexOf(":");
      if (lastColon > lastSlash) return text.slice(0, lastColon) || text;
      return text;
    }

    function buildProxyPayload() {
      return {
        httpProxy: clean(config.httpProxy),
        httpsProxy: clean(config.httpsProxy),
        noProxy: clean(config.noProxy),
      };
    }

    function applyConfig(payload) {
      config.outputDir = payload.outputDir || "";
      config.fileName = payload.fileName || "";
      config.username = payload.username || "";
      config.password = payload.password || "";
      config.httpProxy = payload.httpProxy || "";
      config.httpsProxy = payload.httpsProxy || "";
      config.noProxy = payload.noProxy || "";
    }

    function buildConfigPayload() {
      return {
        outputDir: config.outputDir,
        fileName: config.fileName,
        username: config.username,
        password: config.password,
        httpProxy: config.httpProxy,
        httpsProxy: config.httpsProxy,
        noProxy: config.noProxy,
        defaultTag: selectedTagName.value || "latest",
        platformOs: selectedOs.value || "linux",
        platformArch: selectedArchitecture.value?.architecture || "amd64",
      };
    }

    async function loadConfig() {
      try {
        applyConfig(await invoke("load_app_config"));
      } catch (error) {
        setStatus("warning", "configReadFailedTitle", "readyBody", {}, String(error));
      }
    }

    async function saveConfig(showToast = true) {
      saving.value = true;
      try {
        const saved = await invoke("save_app_config", { config: buildConfigPayload() });
        applyConfig(saved);
        if (showToast) ElMessage.success(t("configSaved"));
      } finally {
        saving.value = false;
      }
    }

    function formatNumber(value) {
      return new Intl.NumberFormat(language.value === "zh" ? "zh-CN" : "en-US").format(Number(value || 0));
    }

    function formatPulls(value) {
      const count = Number(value || 0);
      if (count >= 1000000000) return `${(count / 1000000000).toFixed(1)}B`;
      if (count >= 1000000) return `${(count / 1000000).toFixed(1)}M`;
      if (count >= 1000) return `${(count / 1000).toFixed(1)}K`;
      return String(count);
    }

    function formatBytes(bytes) {
      const value = Number(bytes || 0);
      if (!value) return "0 B";
      const units = ["B", "KB", "MB", "GB"];
      const exponent = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
      return `${(value / 1024 ** exponent).toFixed(exponent === 0 ? 0 : 1)} ${units[exponent]}`;
    }

    function formatTime(date) {
      const pad = (value) => String(value).padStart(2, "0");
      return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
    }

    function resetSelection() {
      selectedRepository.value = null;
      tags.value = [];
      selectedTagName.value = "";
      selectedOs.value = "linux";
      selectedArchLabel.value = "linux/amd64";
    }

    async function searchImages() {
      const query = normalizeSearchKeyword(keyword.value);
      if (!query) {
        ElMessage.warning(t("enterKeyword"));
        return;
      }

      searching.value = true;
      exportResult.value = null;
      resetSelection();
      setStatus("info", "searchingTitle", "searchingBody");
      try {
        await saveConfig(false);
        results.value = await invoke("search_public_images", {
          request: { keyword: query, proxy: buildProxyPayload() },
        });
        setStatus("success", "searchDoneTitle", "searchDoneBody", { count: results.value.length });
      } catch (error) {
        results.value = [];
        setStatus("error", "searchFailedTitle", "readyBody", {}, String(error));
      } finally {
        searching.value = false;
      }
    }

    function syncArchitectureDefaults() {
      const tag = selectedTag.value;
      if (!tag || tag.architectures.length === 0) {
        selectedOs.value = "linux";
        selectedArchLabel.value = "linux/amd64";
        return;
      }
      selectedOs.value = tag.architectures[0].os;
      selectedArchLabel.value = tag.architectures[0].label;
    }

    async function selectImage(row) {
      selectedRepository.value = row;
      loadingTags.value = true;
      tags.value = [];
      selectedTagName.value = "";
      setStatus("info", "loadingImageTitle", "loadingImageBody", { repository: row.repository });
      try {
        const result = await invoke("list_repository_tags", {
          request: { repository: row.repository, proxy: buildProxyPayload() },
        });
        tags.value = result.tags;
        if (tags.value.length) {
          selectedTagName.value = tags.value[0].name;
          syncArchitectureDefaults();
          setStatus("success", "imageSelectedTitle", "imageSelectedBody");
        } else {
          setStatus("warning", "noTagsTitle", "noTagsBody");
        }
      } catch (error) {
        setStatus("error", "loadingImageFailedTitle", "readyBody", {}, String(error));
      } finally {
        loadingTags.value = false;
      }
    }

    function onTagChange() {
      syncArchitectureDefaults();
    }

    function onOsChange() {
      selectedArchLabel.value = archOptions.value[0]?.label || "";
    }

    async function exportImage() {
      if (!selectedRepository.value) {
        ElMessage.warning(t("chooseImageFirst"));
        return;
      }
      if (!selectedTag.value || !selectedArchitecture.value) {
        ElMessage.warning(t("chooseTagArch"));
        return;
      }
      if (!clean(config.outputDir)) {
        ElMessage.warning(t("fillOutputDir"));
        return;
      }

      exporting.value = true;
      exportResult.value = null;
      setStatus(
        "info",
        "exportingTitle",
        "exportingBody",
        {
          image: selectedRepository.value.repository,
          tag: selectedTag.value.name,
          platform: selectedArchitecture.value.label,
        },
      );
      try {
        const result = await invoke("pull_image_as_tar", {
          request: {
            image: selectedRepository.value.repository,
            outputDir: config.outputDir,
            fileName: clean(config.fileName),
            tag: selectedTag.value.name,
            platformOs: selectedArchitecture.value.os,
            platformArch: selectedArchitecture.value.architecture,
            username: clean(config.username),
            password: clean(config.password),
            httpProxy: clean(config.httpProxy),
            httpsProxy: clean(config.httpsProxy),
            noProxy: clean(config.noProxy),
          },
        });
        exportResult.value = {
          imageRef: result.image_ref,
          tarPath: result.tar_path,
          layerCount: result.layer_count,
          finishedAt: formatTime(new Date()),
        };
        await saveConfig(false);
        ElMessage.success(t("exportDoneToast"));
        setStatus("success", "exportDoneTitle", "exportDoneBody");
      } catch (error) {
        setStatus("error", "exportFailedTitle", "readyBody", {}, String(error));
      } finally {
        exporting.value = false;
      }
    }

    async function openExportDirectory() {
      if (!exportResult.value) return;
      try {
        await revealItemInDir(exportResult.value.tarPath);
      } catch (error) {
        ElMessage.error(t("openDirFailed", { error: String(error) }));
      }
    }

    async function browseOutputDir() {
      try {
        const selected = await open({ directory: true, multiple: false, title: t("outputDir") });
        if (selected) config.outputDir = selected;
      } catch (error) {
        ElMessage.error(String(error));
      }
    }

    async function setDefaultOutputDir() {
      if (config.outputDir) return;
      try {
        const home = await homeDir();
        config.outputDir = await join(home, "docker-images");
      } catch {
        // leave empty if home dir cannot be resolved
      }
    }

    onMounted(async () => {
      await loadConfig();
      await setDefaultOutputDir();
    });

    return {
      keyword,
      language,
      languageOptions,
      t,
      setLanguage,
      searching,
      saving,
      exporting,
      loadingTags,
      results,
      selectedRepository,
      tags,
      selectedTagName,
      selectedOs,
      selectedArchLabel,
      exportResult,
      status,
      statusTitle,
      statusBody,
      config,
      selectedTag,
      osOptions,
      archOptions,
      searchImages,
      saveConfig,
      selectImage,
      onTagChange,
      onOsChange,
      exportImage,
      openExportDirectory,
      browseOutputDir,
      formatNumber,
      formatPulls,
      formatBytes,
    };
  },
  template: `
    <main class="app-shell">
      <section class="page-title">
        <div>
          <p>{{ t("appSubtitle") }}</p>
          <h1>{{ t("appTitle") }}</h1>
        </div>
        <div class="language-switch">
          <span>{{ t("language") }}</span>
          <el-segmented
            :model-value="language"
            :options="languageOptions"
            size="small"
            @change="setLanguage"
          />
        </div>
      </section>

      <el-row :gutter="16" class="workspace-row">
        <el-col :xs="24" :lg="9">
          <el-card shadow="hover" class="workspace-card">
            <template #header>
              <div class="card-header-title">{{ t("queryPanel") }}</div>
            </template>

            <el-form label-position="top" class="control-form">
              <el-form-item :label="t('keyword')">
                <div class="keyword-search-field">
                  <el-input v-model="keyword" :placeholder="t('keywordPlaceholder')" clearable @keyup.enter="searchImages" />
                  <el-button type="primary" class="keyword-search-button" :loading="searching" @click="searchImages">{{ t("search") }}</el-button>
                </div>
              </el-form-item>

              <el-form-item :label="t('outputDir')">
                <div class="output-dir-field">
                  <el-input v-model="config.outputDir" :placeholder="t('outputDirPlaceholder')" />
                  <el-button class="output-dir-button" @click="browseOutputDir">{{ t("browse") }}</el-button>
                </div>
              </el-form-item>

              <el-form-item :label="t('fileName')">
                <el-input v-model="config.fileName" :placeholder="t('fileNamePlaceholder')" clearable />
              </el-form-item>

              <el-form-item :label="t('selectedImage')">
                <el-input :model-value="selectedRepository?.repository || ''" :placeholder="t('selectedImagePlaceholder')" readonly />
              </el-form-item>

              <el-form-item :label="t('selectTag')">
                <el-select v-model="selectedTagName" :placeholder="t('selectTagPlaceholder')" :disabled="!tags.length" filterable @change="onTagChange">
                  <el-option
                    v-for="tag in tags"
                    :key="tag.name"
                    :label="tag.name + ' · ' + formatBytes(tag.full_size) + ' · ' + t('architectureCount', { count: tag.architectures.length })"
                    :value="tag.name"
                  />
                </el-select>
              </el-form-item>

              <el-row :gutter="10">
                <el-col :span="12">
                  <el-form-item :label="t('targetOs')">
                    <el-select v-model="selectedOs" :disabled="!osOptions.length" @change="onOsChange">
                      <el-option v-for="os in osOptions" :key="os" :label="os" :value="os" />
                    </el-select>
                  </el-form-item>
                </el-col>
                <el-col :span="12">
                  <el-form-item :label="t('targetArch')">
                    <el-select v-model="selectedArchLabel" :disabled="!archOptions.length">
                      <el-option v-for="arch in archOptions" :key="arch.label" :label="arch.label" :value="arch.label" />
                    </el-select>
                  </el-form-item>
                </el-col>
              </el-row>

              <el-divider>{{ t("proxySection") }}</el-divider>

              <el-form-item label="HTTP_PROXY">
                <el-input v-model="config.httpProxy" placeholder="http://127.0.0.1:10808" clearable />
              </el-form-item>
              <el-form-item label="HTTPS_PROXY">
                <el-input v-model="config.httpsProxy" placeholder="http://127.0.0.1:10808" clearable />
              </el-form-item>
              <el-form-item label="NO_PROXY">
                <el-input v-model="config.noProxy" placeholder="localhost,.corp.local" clearable />
              </el-form-item>

              <el-collapse class="auth-collapse">
                <el-collapse-item :title="t('authSection')" name="auth">
                  <el-form-item :label="t('username')">
                    <el-input v-model="config.username" :placeholder="t('usernamePlaceholder')" clearable />
                  </el-form-item>
                  <el-form-item :label="t('password')">
                    <el-input v-model="config.password" type="password" show-password :placeholder="t('passwordPlaceholder')" clearable />
                  </el-form-item>
                </el-collapse-item>
              </el-collapse>

              <div class="action-bar">
                <el-button :loading="saving" @click="saveConfig(true)">{{ t("saveConfig") }}</el-button>
                <el-button type="success" :loading="exporting" :disabled="!selectedRepository" @click="exportImage">{{ t("export") }}</el-button>
              </div>
            </el-form>
          </el-card>
        </el-col>

        <el-col :xs="24" :lg="15">
          <el-card shadow="hover" class="workspace-card result-card">
            <template #header>
              <div class="result-header">
                <span class="card-header-title">{{ t("results") }}</span>
                <span class="count-text">{{ t("count", { count: results.length }) }}</span>
              </div>
            </template>

            <el-empty v-if="!results.length && !searching" :description="t('emptyResults')" />

            <el-table
              v-else
              :data="results"
              stripe
              height="420"
              highlight-current-row
              v-loading="searching || loadingTags"
              @row-click="selectImage"
            >
              <el-table-column :label="t('imageName')" min-width="230">
                <template #default="{ row }">
                  <div class="image-name">{{ row.repository }}</div>
                  <div class="image-meta">
                    <span>{{ row.is_official ? t("official") : row.namespace }}</span>
                    <span>{{ t("stars") }} {{ formatNumber(row.star_count) }}</span>
                    <span>{{ t("pulls") }} {{ formatPulls(row.pull_count) }}</span>
                  </div>
                </template>
              </el-table-column>
              <el-table-column :label="t('description')" min-width="280">
                <template #default="{ row }">
                  <span class="description-text">{{ row.description || t("noDescription") }}</span>
                </template>
              </el-table-column>
              <el-table-column :label="t('action')" width="120" fixed="right">
                <template #default="{ row }">
                  <el-button type="primary" link @click.stop="selectImage(row)">{{ t("choose") }}</el-button>
                </template>
              </el-table-column>
            </el-table>

            <div class="status-stack">
              <el-alert
                :title="statusTitle"
                :description="statusBody"
                :type="status.type"
                :closable="false"
                show-icon
              />

              <section v-if="exportResult" class="export-result">
                <div class="export-result-head">
                  <div>
                    <h3>{{ t("exportSuccess") }}</h3>
                    <p>{{ t("finishedAt", { time: exportResult.finishedAt }) }}</p>
                  </div>
                  <el-button @click="openExportDirectory">{{ t("openDirectory") }}</el-button>
                </div>

                <div class="result-field">
                  <span>{{ t("image") }}</span>
                  <code>{{ exportResult.imageRef }}</code>
                </div>
                <div class="result-field">
                  <span>{{ t("file") }}</span>
                  <code>{{ exportResult.tarPath }}</code>
                </div>
              </section>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </main>
  `,
};

createApp(App).use(ElementPlus).mount("#app");
