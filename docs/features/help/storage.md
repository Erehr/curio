# Recovery

Curio keeps your originals safe and helps you recover from moved drives, external file edits, interrupted disk writes or database errors. Recovery tools are in **Settings › Storage**, **Settings › Backup** and the **Missing & offline** sidebar view.

## Missing vs offline

Curio separates two unavailability states so unplugged drives never look like deleted files:

| Status | Meaning | What to do |
|---|---|---|
| **Storage offline** | The drive, mount point or network share for an indexed Location is unreachable. | Reconnect the drive or network share. Curio brings its assets back online automatically without a full rescan. |
| **Missing** | The location is online, but the file is no longer at its recorded path. | Click **Relink** in the Inspector, run **Rescan** (Curio matches moved files by hash or inode) or **Relocate** a moved folder. |

## Relocate

- **Relocate a Location**: Open **Settings › Storage** and click **Relocate** beside the Location (or right-click it in the sidebar and choose **Relocate**). Pick the new folder to update every asset path while keeping all tags, folders, ratings, notes and custom fields.
- **Relink one missing file**: Select the missing asset, open **File & location** in the Inspector and click **Relink** (or right-click the asset and choose **More › Locate missing file**). Select the file's new path to relink it without losing metadata.
- **Forget a missing grouped member**: If a grouped companion file (such as an `.xmp` sidecar) was intentionally deleted, click **Forget** next to it in the Inspector's **Files** widget.

## Orphans

`<Library>.curio/files/` is an ordinary folder tree on disk. Files placed directly into `files/` outside Curio, or left behind by interrupted operations, appear as orphaned files with no matching library entry:

1. Open **Settings › Storage** and check the **Files** section.
2. Click **Add back to library** to index those files, or click **Send to Recycle Bin** to remove unwanted stray files.

## Pending ops

Managed file placements (`files/` hard links, moves and renames) are tracked in a crash-safe placement journal inside SQLite. If antivirus software, a locked file handle or power loss interrupts a file operation:

1. Open **Settings › Storage** and check the **Files** section.
2. Close any program holding the file open and click **Retry** (or **Retry all**) to finish the operation.
3. Click **Forget** (or **Forget all**) to clear a stuck entry if the file is gone or no longer needed.

## Database

In **Settings › Backup › Maintenance**:

- **Check integrity**: Runs SQLite's integrity check on `library.db` and reports whether the database structure and indexes are healthy.
- **Optimize**: Compacts unused space and refreshes query planner statistics after large imports or purges.

## Restore

If `library.db` is damaged or you need to roll back a bulk change:

1. Open **Settings › Backup › Database backups** and click **Restore a backup**.
2. Review the available manual, automatic, pre-migration and before-restore snapshots with their dates, schema versions and sizes.
3. Click **Restore** on a snapshot, or click **Choose file** to pick a backup from another folder.
4. Confirm the restore. Curio saves a safety copy of your current database before swapping in the backup and restarting.
5. Run **Rescan** on your Locations and check **Files** in **Settings › Storage** for any orphaned files. See [Back up and restore](../libraries/backup.md).
