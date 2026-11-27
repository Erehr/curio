# Text & links

Curio opens plain text, Markdown, structured data, common source code files and saved web links inside the viewer. Text preferences (size, line wrapping, line numbers and Markdown mode) persist across files, while web links render in an embedded YouTube player or an isolated web page view.

## Text & JSON

The built-in text viewer opens `.txt`, `.md`, `.markdown`, `.json`, `.csv`, `.tsv`, `.log`, `.xml`, `.yaml`, `.yml`, `.ini`, `.toml`, `.js`, `.py`, `.rs`, `.css`, `.scss`, `.sh`, `.bat`, `.ps1`, `.c`, `.cpp`, `.h`, `.java`, `.go`, `.lua`, `.sql`, `.srt` and `.vtt` files. The Inspector's compact preview displays the first 3,000 characters.

- **Formatted JSON**: Valid `.json` files format automatically with 2-space indentation; malformed JSON displays as raw text.
- **Wrap lines**: Toggle **Wrap lines** in the viewer toolbar or press ++alt+z++. Unwrapped lines scroll horizontally; wrapped lines break within the reading column.
- **Line numbers**: Toggle **Line numbers** in the viewer toolbar or press ++l++ to show a line-number gutter alongside raw text and code.
- **Text scaling**: Scale text between **50%** and **400%** using the toolbar zoom slider, ++ctrl+equal++ / ++ctrl+minus++, ++ctrl++ + mouse wheel or right-drag up and down. Press ++ctrl+0++ or click **Actual size** to reset to 100%.
- **Scrolling and selection**: The mouse wheel scrolls normally, while left-drag and double-click select text for copying. Press ++page-up++, ++page-down++, ++home++ and ++end++ to navigate (manually grouped files take precedence for ++page-up++ and ++page-down++). Changing zoom, wrap, line numbers or Markdown mode preserves your scroll position.

!!! note "Other Code extensions"
    Curio indexes many programming languages under the **Code** asset type (including `.html`, `.ts`, `.tsx`, `.vue`, `.php`, `.rb`, `.kt`, `.swift` and `.zig`) and draws text-card thumbnails for them. Extensions outside the built-in text viewer list open in your default editor with ++shift+alt+o++ or middle-click.

## Markdown

When viewing `.md` or `.markdown` files, switch between three modes in the viewer toolbar or cycle with ++m++:

- **Rendered**: Parses Markdown (up to 400,000 characters) into headings, tables, task lists, blockquotes and code blocks styled with your active Curio theme.
- **Source**: Shows raw Markdown source in the monospace pane, with optional **Wrap lines** (++alt+z++) and **Line numbers** (++l++).
- **Side by side**: Shows source on the left and rendered output on the right with synchronized scrolling.

Rendered Markdown passes through an HTML allowlist for safety:

- Scripts, `<style>`, `<iframe>`, forms and embedded `<svg>`, `<video>` and `<audio>` elements are stripped. Untrusted `.md` files cannot run code, alter Curio's styles or make network requests.
- Only inline `data:image/*;base64,...` images render; remote or relative image URLs display their alt text or path instead of fetching.
- Clicking a `#heading` anchor scrolls to that heading; clicking an `http://` or `https://` link opens it in your default browser.

## Links

Saved web links (`.url` on Windows and `.webloc` on macOS, created when you paste, drop or save URLs into Curio) belong to the **Links** asset type.

- **Web pages in the viewer**: Opening an `http://` or `https://` link loads the live page in an isolated WebView overlay. It shares Curio's page browser profile (keeping cookies and consent banners) while blocking access to Curio's internal APIs. When a Curio menu, popover or dialog overlaps the stage, the WebView steps aside and displays the link's thumbnail so UI controls stay visible.
- **Close from inside a page**: Press ++escape++ even while focus is inside the embedded web page to return to the library.
- **Open externally**: Click **Open with default app** (++shift+alt+o++) in the toolbar, or choose **Open in browser** from the context menu. Non-HTTP links and fallback views show a floating **Open in app** and **Browser** bar.

## Link previews

Link thumbnails stay opt-in for privacy:

- **Settings › Preview › Web links › Link previews** is off by default so scanning `.url` or `.webloc` files never contacts external websites on its own.
- When enabled, Curio captures a screenshot using a hidden WebView sharing your Curio page profile. If that fails, it tries headless Microsoft Edge or Google Chrome (overridable with the `CURIO_BROWSER` environment variable), then falls back to the page's Open Graph `og:image` or site icon.
- Links pointing to `localhost` or private local-network addresses never capture thumbnails.
- **YouTube links**: Supported YouTube URLs (`youtube.com/watch?v=`, `youtu.be/`, `/shorts/`, `/embed/`, `/live/`, `/v/` and `youtube-nocookie.com/embed/`) fetch the video poster (`maxres`, falling back to `hq` 480x360). They open in an embedded privacy-enhanced player (`youtube-nocookie.com`) with autoplay.
- Refresh a link's preview thumbnail at any time from the context menu using **More › Refresh thumbnail**.
