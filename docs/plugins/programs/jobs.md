# Jobs

Run multi-second or multi-minute work such as media transcoding, playlist downloads, batch image conversion or external CLI pipelines as a **job**. Jobs display progress in Curio's persistent toast bar, keep running if a plugin window closes and let the user cancel the work at any time.

## External tools

Instead of hardcoding executable paths or searching the filesystem, declare supported CLI tools in `plugin.json` under `tools[]`:

```json
{
  "permissions": ["files.write", "exec"],
  "tools": ["yt-dlp", "ffmpeg"]
}
```

Query [`tool.list`](../../api/operations/tool.md#tool-list) at runtime to check which tools are installed and direct users to **Settings › Plugins › Tools** (or the tool's `homepage`) when one is missing.

## Supported tools

Curio locates seven standard CLI tools by checking the user's custom override in **Settings › Plugins › Tools**, Curio's bundled FFmpeg (for `ffmpeg` and `ffprobe` on Windows), system `PATH` and standard package manager directories (WinGet, Scoop, Chocolatey, Homebrew and `~/.local/bin`):

| Tool `name` | Display title | Purpose |
|---|---|---|
| `ffmpeg` | FFmpeg | Video, audio and animated image conversion |
| `ffprobe` | FFprobe | Media container and stream inspection |
| `yt-dlp` | yt-dlp | Downloading video and audio from web pages |
| `gallery-dl` | gallery-dl | Downloading image galleries and collections |
| `exiftool` | ExifTool | Photo, video and document metadata reading and writing |
| `magick` | ImageMagick | Command-line image conversion and processing |
| `7z` | 7-Zip | Archive inspection and extraction |

Curio never downloads third-party tools. Users install them or point Curio at an existing binary in **Settings › Plugins › Tools**.

## process.run

With `exec`, call [`process.run`](../../api/operations/process.md#process-run) from any plugin page or program to spawn a declared `tool` or a bundled `program`:

```javascript
const { job, folder } = await curio.process.run({
  tool: 'yt-dlp',
  args: ['--newline', '-o', '%(title)s.%(ext)s', url],
  title: 'Downloading video',
})
```

## Run folder

Every `process.run` invocation provides:

- **Temporary working directory**: Returned as `folder` and passed in the child environment as `CURIO_RUN_FOLDER`. Curio deletes the directory 24 hours later.
- **`CURIO_JOB`**: Active job ID, letting the spawned script call `job.update` to report percentage progress to Curio's progress bar.
- **API credentials**: `CURIO_API_URL`, `CURIO_API_TOKEN` and `CURIO_PLUGIN` for live API calls.

## Streaming output

- **Short runs**: Pass `wait: true` (waits up to 120 seconds) to receive `{ state, code, stdout, stderr, folder }` in one `await`.
- **Long runs**: Leave `wait: false` and listen to [`job.output`](../../api/events/jobs.md#job-output) (lines split on both `\n` and `\r` so CLI progress bars stream live) and [`job.finished`](../../api/events/jobs.md#job-finished).

```javascript
const sub = curio.subscribe({ events: ['job.output', 'job.finished'] }, async (ev) => {
  if (ev.name === 'job.output' && ev.data.job === job) {
    appendLog(ev.data.lines)
  }
  if (ev.name === 'job.finished' && ev.data.id === job) {
    sub.close()
    if (ev.data.state === 'done') console.log('Finished:', ev.data.result)
  }
})
```

## Progress & cancel

When your plugin runs multi-step work inside a page without `process.run` (for example batch Canvas image conversion), track progress with [`job.start`](../../api/operations/job.md#job-start), [`job.update`](../../api/operations/job.md#job-update) and [`job.finish`](../../api/operations/job.md#job-finish):

```javascript
const { job } = await curio.job.start({ title: 'Converting images', total: items.length })
try {
  for (let i = 0; i < items.length; i++) {
    const { cancelled } = await curio.job.update({ job, done: i, text: items[i].name })
    if (cancelled) return
    await convertOne(items[i])
  }
  await curio.job.finish({ job, result: { count: items.length } })
} catch (err) {
  await curio.job.finish({ job, error: err.message })
}
```

When a user clicks Cancel on a `process.run` job (or your plugin calls [`job.cancel`](../../api/operations/job.md#job-cancel)), Curio terminates the process and its entire child process tree immediately.

## Import & cleanup

Import files written inside `folder` (`CURIO_RUN_FOLDER`) into the library with [`item.add.fromPaths`](../../api/operations/item-add.md#item-add-frompaths) (`move: true`) using only `files.write`. Curio allows importing from the run folder without `files.outside` while the job runs and for 10 minutes after it completes:

```javascript
await curio.item.add.fromPaths({
  paths: [`${folder}/output.mp4`],
  move: true,
  folders: place.folders,
  tags: place.tags,
  wait: true,
})
```
