# plugin.json

Top-level manifest schema for `plugin.json`. Place `plugin.json` at the root of your plugin folder. Optional translated strings for manifest labels live beside it in `locales/<lang>.json`.

## id {#id}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Globally unique plugin identifier (for example `"curio.reader"` or `"com.example.converter"`). Used for per-plugin page origins (`plugin://` on Linux and macOS, `http://plugin.<id>.localhost/` on Windows), storage isolation, dependency matching and rule or reader IDs. Plugins that serve HTML pages must use only ASCII letters, digits, `.`, `-` and `_` (not starting with `.`).

## name {#name}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Human-readable display name shown in **Settings › Plugins**, the **Plugins** menu and **Settings › Shortcuts** groups.

## version {#version}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Semantic version string (for example `"1.0.0"`) displayed in **Settings › Plugins** and checked against other plugins' `dependencies`.

## api {#api}
<div class="curio-meta"><span class="curio-badge">integer</span><span class="curio-badge curio-badge--muted">Default: 1</span></div>
Manifest API version required by this plugin. Currently `1`; versions newer than what Curio supports prevent activation.

## min_curio {#min_curio}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Minimum Curio version required (for example `"1.2.0"`), for plugins that rely on API operations, tools or manifest options added within the same `api` version. Older Curio builds keep the plugin inactive and report the required version.

## description {#description}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
One-sentence summary shown beneath the plugin name in **Settings › Plugins**.

## author {#author}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Author or organization name.

## icon {#icon}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Tabler icon name without prefix (for example `"book"`, `"cube"` or `"sparkles"`) shown in **Settings › Plugins** and the **Plugins** menu.

## dependencies {#dependencies}
<div class="curio-meta"><span class="curio-badge">Record&lt;string, string&gt;</span><span class="curio-badge curio-badge--muted">Default: {}</span></div>
Map of required plugin IDs to version requirements (such as `"1.2"`, `">=1.0.0"`, `"^1"`, `"=1.0.0"` or `"*"`). Missing, incompatible or disabled dependencies keep this plugin inactive.

## permissions {#permissions}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Requested permissions: `metadata.read`, `metadata.write`, `organize`, `files.read`, `files.write`, `files.outside`, `ui`, `exec`, `network`. `library.read` is always granted implicitly; `library.manage` is never allowed. See [Permissions and isolation](../start/permissions.md).

## fields {#fields}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Typed custom metadata fields registered by this plugin. See [Field declarations](fields.md).

## extractors {#extractors}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Declarative (`png_text`, `regex`, `json`), hidden-page (`page`) or CLI (`command`) metadata readers. See [Extractor declarations](extractors.md).

## widgets {#widgets}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Inspector and sidebar widgets (field groups or HTML pages). See [Widget declarations](widgets.md).

## actions {#actions}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Commands placed in item menus, the **Plugins** menu, the viewer, shortcuts or reusable action steps. See [Action and URL handler declarations](actions.md).

## urls {#urls}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
URL pattern handlers claiming dropped, pasted, extension-captured or API-added web addresses. See [Action and URL handler declarations](actions.md#urls-id).

## filters {#filters}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Named one-click Filter bar presets backed by query language expressions. See [Search, sort and column declarations](discovery.md).

## sorts {#sorts}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Named **Sort by** options backed by custom fields. See [Search, sort and column declarations](discovery.md#sorts-label).

## panels {#panels}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Declarative Library sidebar panels containing text and one-click filter links. See [Window, tab and panel declarations](surfaces.md#panels-id).

## columns {#columns}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Specialized host-rendered List view columns (such as `render: "font"`). See [Search, sort and column declarations](discovery.md#columns-id).

## formats {#formats}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
File extension handlers, custom display types (`type`), viewers, thumbnailers, screens, companion rules and open-as-filter rules. See [Format and companion declarations](formats.md).

## windows {#windows}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Standalone plugin windows. See [Window, tab and panel declarations](surfaces.md).

## tabs {#tabs}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Workspace tabs mounted beside Curio's item grid and Tag Manager. See [Window, tab and panel declarations](surfaces.md#tabs-id).

## viewer_tools {#viewer_tools}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Native toolbar controls (buttons, sliders, text readouts) driven by viewer pages via `viewer.setTools`. See [Viewer screens, tools and keys](viewers.md#viewer_tools-id).

## viewer_keys {#viewer_keys}
<div class="curio-meta"><span class="curio-badge">object[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Viewer-scoped rebindable shortcuts delivered via `plugin.tool`. See [Viewer screens, tools and keys](viewers.md#viewer_keys-id).

## tools {#tools}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Declared external CLI tools discoverable via `tool.list` and runnable via `process.run` (`ffmpeg`, `ffprobe`, `yt-dlp`, `gallery-dl`, `exiftool`, `magick`, `7z`).

## network {#network}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Host allowlist for page network access (`connect-src`, `img-src` and `media-src` CSP) when the `network` permission is granted (for example `["openlibrary.org", "*.openstreetmap.org"]`). Empty allows any `https:`/`wss:` host when `network` is granted.

## settings {#settings}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Relative path to an HTML settings page embedded inline inside **Settings › Plugins** (for example `"settings.html"`).

## theme_variables {#theme_variables}
<div class="curio-meta"><span class="curio-badge">Record&lt;string, string&gt;</span><span class="curio-badge curio-badge--muted">Default: {}</span></div>
Custom CSS variables published as `--plugin-<name>` (`.` and `_` become `-`). Values must use only ASCII letters, digits, spaces and `# % . , + - * / ( )` with balanced parentheses and no `url(`. Ignored in `--safe-mode`.
