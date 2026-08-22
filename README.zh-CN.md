<p align="center">
  <img src="assets/quotastrip-mark.svg" width="64" height="64" alt="QuotaStrip 标识">
</p>

<h1 align="center">QuotaStrip</h1>

<p align="center"><a href="README.md">English</a> | <a href="README.zh-CN.md">简体中文</a></p>

<p align="center"><strong>让 Codex 用量始终一眼可见。</strong><br>一个始终贴合 Codex 窗口的 Windows 原生 Notch。</p>

<p align="center">
  <a href="https://github.com/yangy-ai-2026/quotastrip/releases">发布版本</a>&nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="#工作原理">工作原理</a>&nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="https://github.com/yangy-ai-2026/quotastrip/issues">Issues</a>&nbsp;&nbsp;·&nbsp;&nbsp;
  <a href="https://github.com/yangy-ai-2026/quotastrip">GitHub</a>
</p>

QuotaStrip 会将一个紧凑的用量 Notch 贴合在 Windows 上的 Codex Desktop 窗口顶部，让账户额度信息始终靠近你的工作位置。

<p align="center">
  <img src="assets/quotastrip-hero-readme.png" width="900" alt="QuotaStrip 贴合在 Codex 窗口顶部中央">
</p>

## 核心特性

<sub>四项原生行为，一个贴合窗口的使用状态界面。</sub>

<table>
  <tr>
    <td width="50%" valign="top">
      <strong>01&nbsp; / &nbsp;一眼可见</strong><br><br>
      无需打断当前工作，即可在 Codex 窗口附近查看用量。
    </td>
    <td width="50%" valign="top">
      <strong>02&nbsp; / &nbsp;悬停展开</strong><br><br>
      展开 Notch，查看剩余百分比、重置时间和数据状态。
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top">
      <strong>03&nbsp; / &nbsp;原生贴合</strong><br><br>
      跟随 Codex 的移动、缩放、最大化/还原和显示器变化。
    </td>
    <td width="50%" valign="top">
      <strong>04&nbsp; / &nbsp;本地优先隐私</strong><br><br>
      不收集提示词（prompts）、对话或项目代码，也没有隐藏遥测。
    </td>
  </tr>
</table>

## 截图

### Compact

<sub>一条紧凑的状态界面，始终贴合在窗口顶部。</sub>

<p align="center">
  <img src="assets/quotastrip-compact.png" width="900" alt="贴合在 Codex 窗口上的 QuotaStrip 紧凑状态">
</p>

### Hover

<sub>悬停时可在不离开窗口的情况下查看用量状态。</sub>

<p align="center">
  <img src="assets/quotastrip-hover.png" width="900" alt="贴合在 Codex 窗口上的 QuotaStrip 悬停状态">
</p>

### Click

<sub>点击后打开详细的额度信息视图。</sub>

<p align="center">
  <img src="assets/quotastrip-click.png" width="900" alt="贴合在 Codex 窗口上的 QuotaStrip 点击详情状态">
</p>

## 安装

> [!NOTE]
> QuotaStrip `v0.1.0` 已正式发布，可从 [QuotaStrip v0.1.0](https://github.com/yangy-ai-2026/quotastrip/releases/tag/v0.1.0) 下载。

请下载 Windows NSIS 安装程序 `QuotaStrip_0.1.0_x64-setup.exe`，并在安装前使用 [SHA256SUMS.txt](https://github.com/yangy-ai-2026/quotastrip/releases/download/v0.1.0/SHA256SUMS.txt) 验证 SHA256 校验和。`v0.1.0` 未签名，Windows SmartScreen 可能显示警告；请仅从官方 Release 下载并验证 SHA256 校验和。

## 工作原理

1. QuotaStrip 查找当前的 Codex Desktop 窗口。
2. Windows 原生悬浮层（Overlay）被放置在该窗口的顶部中央，并跟踪窗口几何变化。
3. 用量数据通过 Codex 自有的本地能力读取和呈现，不收集提示词、对话或项目代码。

## 架构

```text
Codex 自有本地用量来源
        ↓
QuotaStrip 用量 / 运行时层
        ↓
Windows 原生悬浮层与窗口跟踪
        ↓
Compact / Hover / Click 界面
```

QuotaStrip 通过 Codex 自有的本地能力读取额度数据，再经用量 / 运行时层处理，并呈现在贴合 Codex 窗口的 Windows 原生悬浮层中。Compact、Hover 和 Click 是同一界面的不同呈现状态。本地优先的隐私边界明确排除提示词、对话、项目代码以及由应用管理的凭据。

## Windows 要求

- Windows 10 或 Windows 11
- Codex Desktop

QuotaStrip 以 Windows 为先；macOS 和 Linux 不在 `v0.1.0` 范围内。

## 隐私

QuotaStrip 采用本地优先设计，不收集提示词（prompts）、对话或项目代码。它不会请求 OpenAI 密码、管理 Codex 凭据、使用 OCR 或屏幕抓取，也不会发送隐藏遥测。

只有在当前账户实际返回额度窗口时，QuotaStrip 才会显示它们。它不会假定固定的额度窗口，也不会编造缺失的用量数据。

## 常见问题

### QuotaStrip 需要 OpenAI API key 吗？

不需要。认证仍由 Codex 管理；QuotaStrip 不会请求或管理 OpenAI 凭据。

### 它会读取我的提示词或对话吗？

不会。QuotaStrip 不收集提示词、对话或项目代码。

### 支持 macOS 或 Linux 吗？

不支持。`v0.1.0` 以 Windows 为先，macOS 和 Linux 不在当前范围内。

### Codex 关闭后会怎样？

QuotaStrip 需要检测到 Codex Desktop 窗口才能贴合悬浮层。重新打开 Codex 后，窗口跟踪器可以贴合到新的窗口。

### `v0.1.0` 将从哪里下载？

请从 [QuotaStrip v0.1.0](https://github.com/yangy-ai-2026/quotastrip/releases/tag/v0.1.0) 下载，这是官方 GitHub Release。

### 为什么 Windows 可能显示 SmartScreen 警告？

`v0.1.0` 未签名。请仅从官方 GitHub Release 下载，并在信任安装程序前验证已发布的 SHA256 校验和。

## 故障排查

### QuotaStrip 没有显示

确认 Codex Desktop 正在运行且窗口可用。然后打开 QuotaStrip 托盘菜单并选择 **Show Overlay**。

### 托盘图标存在，但悬浮层未显示

先选择 **Show Overlay**。如果仍未显示，请从托盘菜单完全退出 QuotaStrip 后重新启动，并确认 Codex Desktop 已打开。

### 我需要验证安装程序

请仅从 [QuotaStrip v0.1.0](https://github.com/yangy-ai-2026/quotastrip/releases/tag/v0.1.0) 获取安装程序，并与随附的 `SHA256SUMS.txt` 文件比对。

### Windows 对安装程序发出警告

请谨慎对待 SmartScreen 或签名警告。只有在安装程序来自官方 GitHub Releases 页面，且其已发布的 SHA256 校验和匹配时，才应继续信任该安装程序。

## 验证与当前限制

Windows CI 工作流会验证版本一致性、构建 Windows 应用和 NSIS 安装程序，并验证安装程序产物及 SHA256 校验和。

`v0.1.0` 已面向 Windows 发布。二进制文件未签名，Windows SmartScreen 可能显示警告；请在安装前验证 SHA256 校验和。

## 路线图

- `v0.1.0` — Codex 用量引擎、贴合 Windows 窗口的 Notch、悬停用量面板、托盘控制，以及可复现的 Windows NSIS 安装程序。
- 后续 — 稳定性改进、更广泛的 Windows 兼容性，以及由用户反馈驱动的增强。

请参阅 [ROADMAP.md](ROADMAP.md) 了解当前公开路线图。

## 参与贡献

欢迎专注的贡献。在提交 Issue 或 Pull Request 前，请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 安全

如需报告漏洞或了解披露流程，请参阅 [SECURITY.md](SECURITY.md)。

## 许可证

QuotaStrip 采用 [MIT License](LICENSE) 授权。

## 免责声明

**面向 Windows 上 Codex 的独立开源工具。与 OpenAI 无隶属关系，也未获 OpenAI 认可或背书。**

“Codex”和“OpenAI”仅用于标识本工具所适配的产品。本仓库不声称获得官方 API 支持、认证、赞助或背书。
