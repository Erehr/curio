# Terms

Curio uses distinct words for where files live, how assets are grouped, how windows are arranged and how plugins extend the app. Knowing these terms avoids surprises when managing files on disk, writing queries or calling the API.

## Library & assets

- **Library**: A `.curio` folder on disk containing `library.db` (the SQLite index), thumbnail and preview caches, database backups and a `files/` folder tree. One Curio process holds one open library at a time; you can run multiple libraries simultaneously in separate windows.
- **Asset (Item)**: One catalog entry in a library with a stable UUID (`id`), title, rating, favorite and explicit flags, notes, source URL, tags, folder memberships, custom fields and color palette. User documentation calls this an **asset**; the API and plugin references call it an **item** to match the `item.*` namespace.
- **File**: An actual file on disk referenced by an asset. Most assets point to one file; **grouped files** attach one or more companion or manually grouped files to a primary file under one asset.

## Folders & locations

| Term | What it is | Disk effect |
|---|---|---|
| **Folder** | A container inside Curio's **Folders** tree. One asset can belong to zero, one or many folders at once. | Curio mirrors every folder as a real directory inside `<Library>.curio/files/`. Managed assets live there as hard links; indexed location assets receive a hard link (or a copy across drives) inside `files/` while their originals stay untouched in their location. |
| **Smart folder** | A saved live query built visually with rule groups or written in the [Query language](../api/query/syntax.md). Can nest inside another smart folder to narrow it and optionally generate **Automatic subfolders**. | None; computed from the database index. |
| **Location** | An existing directory on your computer, external drive or mounted network share indexed **in place** without copying its files into the library. Has an access level (**Read-only**, **Edit files** or **Edit and organize**). | Originals stay where they are. Curio watches for external changes and only edits, renames or moves location files when the access level allows it. |
| **Directory** | A real filesystem subdirectory inside an indexed **Location** (shown when expanding a Location in the sidebar or viewing location subfolders). | Maps 1:1 to a folder on disk. |

## Tags & fields

- **Tag**: A freeform keyword attached to any number of assets. Tags can be starred, colored, merged, renamed or applied automatically through folder **auto-tags**.
- **Tag group**: A named collection of tags (with optional color, icon and description) used to organize tags in the **Tag Manager** and tag pickers. A tag can belong to multiple tag groups and inherits its primary group's color when it has no color of its own.
- **Custom field**: A typed metadata attribute (text, integer, float, boolean, date, date/time, single choice, multiple choice, JSON, URL or file reference) attached to assets. Fields can be **user-defined** (in **Settings › Fields** or via `field.define`), **built-in** or **plugin-owned** (declared in `plugin.json`). A field can also be **paired** with another field (such as `LoRA` paired with `LoRA strength`, or `Series` paired with `Number in series`).
- **Reader (Extractor)**: A built-in or plugin component that reads metadata from files (EXIF camera info, audio tags, font metrics, PNG text chunks, companion JSON files, HTML extractor pages or CLI commands) and writes values into indexed fields. Manual edits override reader output so rescans keep your edits.

## Grouping

Three separate features use the word *group* in Curio:

1. **Grouped files (companions and manual groups)**: Multiple files on disk combined into **one asset** (such as RAW + JPEG, Live Photo HEIC + MOV, OBJ + MTL, model weight + `.civitai.info` + preview image or manually grouped files). Configured in **Settings › Grouped files**, plugin `formats[].companions` rules or `item.group` / `item.split`.
2. **Display groups (`Group by`)**: Collapsible section headers in Grid, Masonry, Justified and List views that sort assets by Type, Extension, Date, Rating, Size, Name, Location or an indexed custom field. Plugins can add custom Type labels (such as `Books` or `Models`). Display grouping never changes files or asset identity.
3. **Tag groups**: Buckets for organizing tags in the Tag Manager; unrelated to files or view grouping.

## Layout & widgets

- **Zone**: A dockable region of the Curio window: top **Toolbar**, **Library** sidebar, **Filter bar**, **Inspector** and viewer toolbars.
- **Widget (Part)**: An individual control or panel placed inside a zone (such as Breadcrumb, Search, Rating, Properties, Colors or a plugin widget or viewer button).
- **Screen**: A window mode with its own layout: **Main** browsing, **Folder**, **Tags** or a **Viewer** screen (image, video, audio, font or a plugin-declared screen such as Reader or 3D models). The Library sidebar zone is shared across browsing screens.
- **Layout**: A saved arrangement of zones, dock positions and widgets. Curio keeps separate layouts for landscape and portrait window shapes, and optionally per library.
- **Window kind**: Curio opens five kinds of windows: **main** library windows, standalone **asset** windows, built-in **tool** windows (Batch rename, Find duplicates, Compare), the **Settings** window and **plugin** windows.

## Plugins & actions

- **Plugin**: A folder containing `plugin.json` that adds formats, fields, metadata readers, filters, sorts, columns, widgets, tabs, windows, viewers, actions or URL handlers. Curio ships built-in format families, offers three optional official plugins (**Reader**, **3D models** and **AI generation**) and loads third-party plugins added in **Settings › Plugins**.
- **Page**: An HTML/CSS/JavaScript document inside a plugin served in a sandboxed iframe (`plugin://` on Linux and macOS, `http://plugin.<id>.localhost/` on Windows) for a widget, tab, window, viewer, settings page, popover or hidden thumbnail/extractor worker.
- **Program**: A native executable or script launched by Curio for a plugin action, URL handler, metadata extractor, thumbnailer or `process.run` job, authenticated back to Curio's API via a short-lived scoped run token (`CURIO_API_TOKEN`).
- **Tool**: An external CLI program Curio discovers for plugins (`ffmpeg`, `ffprobe`, `yt-dlp`, `gallery-dl`, `exiftool`, `magick`, `7z`), configured in **Settings › Plugins › Tools** and listed by `tool.list`.
- **Action**: Either a user-created multi-step workflow ([Build reusable actions](../features/organize/actions.md)) or a plugin-contributed command (`actions[]` in `plugin.json`) available in item menus, toolbars, viewers, shortcuts or action steps.
