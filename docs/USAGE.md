# YARW: Yet Another Rsync for Windows - Detailed Usage Guide

This document provides comprehensive usage information for YARW, including detailed explanations of all options and features.

For additional questions or issues, please refer to the project repository or ask ChatGPT/Gemini/Claude.

## Table of Contents

- [Basic Syntax](#basic-syntax)
- [Common Usage Patterns](#common-usage-patterns)
- [Option Reference](#option-reference)
  - [Basic Options](#basic-options)
  - [Windows Unsupported Options](#windows-unsupported-options)
  - [Transfer Options](#transfer-options)
  - [Delete Options](#delete-options)
  - [Filtering Options](#filtering-options)
  - [Output Options](#output-options)
  - [Backup Options](#backup-options)
  - [Control Options](#control-options)
  - [Checksum Options](#checksum-options)
  - [Remote Transfer Options](#remote-transfer-options)
  - [Daemon Mode Options](#daemon-mode-options)
- [Advanced Usage](#advanced-usage)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)

## Basic Syntax

```bash
rsync [OPTIONS] SOURCE... DESTINATION
```

- **SOURCE**: One or more source paths (files or directories)
- **DESTINATION**: Destination path (file or directory)

**Important**: When syncing directory contents, use a trailing slash on the source:

```bash
# Sync contents of source_folder into destination_folder
rsync -av source_folder/ destination_folder

# Without trailing slash, creates destination_folder/source_folder
rsync -av source_folder destination_folder
```

## Common Usage Patterns

### Pattern 1: Basic Directory Sync

Synchronize two directories:

```bash
rsync -av source/ destination/
```

Options explained:
- `-a`: Archive mode (recursive + preserve links)
- `-v`: Verbose output

### Pattern 2: Backup with Delete

Create an exact mirror, deleting files that don't exist in source:

```bash
rsync -av --delete source/ destination/
```

### Pattern 3: Incremental Backup

Only update files that have changed:

```bash
rsync -avu source/ destination/
```

The `-u` flag skips files that are newer on the destination.

### Pattern 4: Preview Changes (Dry Run)

See what would be transferred without making changes:

```bash
rsync -avn --delete source/ destination/
```

### Pattern 5: Detailed Progress

Show detailed progress and file-by-file changes:

```bash
rsync -avi --progress --stats source/ destination/
```

Options:
- `-i`: Itemize changes (show what's being done to each file)
- `--progress`: Show transfer progress
- `--stats`: Show transfer statistics at the end

## Option Reference

### Basic Options

#### `-v, --verbose`

Increase verbosity. Can be specified multiple times for more detail:

```bash
rsync -v source/ dest/        # Basic verbosity
rsync -vv source/ dest/       # More detailed
rsync -vvv source/ dest/      # Very detailed (debug level)
```

Level 1 (`-v`): Shows files being transferred
Level 2 (`-vv`): Shows files being skipped, additional details
Level 3 (`-vvv`): Debug-level information

#### `-q, --quiet`

Suppress non-error messages. Useful in scripts where you only want to see errors:

```bash
rsync -aq source/ dest/
```

#### `-a, --archive`

Archive mode - the most commonly used option. On Windows, equivalent to `-rl`:

- `-r`: Recursive
- `-l`: Copy symlinks as symlinks

On Unix, `-a` would also include `-ptgoD` (permissions, times, group, owner, devices), but **YARW's `-a` on Windows does NOT automatically enable these options** because they are not supported on Windows. If you explicitly use `-p`, `-t`, `-g`, `-o`, or `-D` options, they will be accepted but will display a warning and be ignored.

```bash
rsync -a source/ dest/
```

**Windows-specific behavior:**
- `-a` = `-rl` (not `-rlptgoD` like on Unix)
- Options `-p`, `-t`, `-g`, `-o`, `-D` can be specified but will be ignored with a warning
- Use `-a` for most common synchronization tasks on Windows

#### `-r, --recursive`

Recurse into directories. Required for copying directory trees:

```bash
rsync -r source/ dest/
```

Note: `-a` includes `-r`, so you don't need both.

#### `-R, --relative`

Use relative path names. Preserves the directory structure:

```bash
# Creates destination/path/to/file.txt
rsync -aR path/to/file.txt destination/
```

#### `-u, --update`

Skip files that are newer on the receiver. Useful for incremental backups:

```bash
rsync -au source/ dest/
```

This prevents overwriting files that have been updated on the destination.

#### `-c, --checksum`

Use checksums instead of file size and modification time to determine if files need updating:

```bash
rsync -ac source/ dest/
```

**When to use:**
- When you need to ensure exact file content matching
- When file timestamps are unreliable
- For validation after a previous transfer

**Trade-off:** Slower, as it reads entire file contents to compute checksums.

#### `-l, --links`

Copy symlinks as symlinks:

```bash
rsync -al source/ dest/
```

On Windows, this has limited support due to symlink restrictions.

#### `-L, --copy-links`

Transform symlinks into the files/directories they reference:

```bash
rsync -aL source/ dest/
```

#### `-H, --hard-links`

Preserve hard links. Files that are hard-linked together in the source will be hard-linked together in the destination:

```bash
rsync -aH source/ dest/
```

**Note:** Partial support on Windows.

#### `--help`

Display help information:

```bash
rsync --help
```

Shows a summary of all available options and their descriptions.

### Windows Unsupported Options

The following options are parsed but not supported on Windows. Using these options will display a warning message and they will be ignored:

#### `-p, --perms`

Preserve file permissions.

```bash
rsync -ap source/ dest/
```

**Windows Note:** File permissions are handled differently on Windows (ACLs vs Unix permissions). This option is ignored on Windows and will display a warning.

#### `-o, --owner`

Preserve file owner.

```bash
rsync -ao source/ dest/
```

**Windows Note:** Not supported on Windows. Will display a warning and be ignored.

#### `-g, --group`

Preserve file group.

```bash
rsync -ag source/ dest/
```

**Windows Note:** Not supported on Windows. Will display a warning and be ignored.

#### `-t, --times`

Preserve modification times.

```bash
rsync -at source/ dest/
```

**Windows Note:** Not supported on Windows. Will display a warning and be ignored.

#### `-D`

Preserve device and special files (equivalent to `--devices --specials`).

```bash
rsync -aD source/ dest/
```

**Windows Note:** Not supported on Windows. Will display a warning and be ignored.

#### `--devices`

Preserve device files.

**Windows Note:** Not supported on Windows. Will display a warning and be ignored.

#### `--specials`

Preserve special files.

**Windows Note:** Not supported on Windows. Will display a warning and be ignored.

**Important:** These options are traditionally included in `-a` (archive mode) on Unix systems, but YARW's `-a` on Windows is equivalent to only `-rl` (recursive + links) to avoid unnecessary warnings. You can still use these options explicitly, but they will trigger warnings and be ignored.

### Transfer Options

#### `-z, --compress`

Compress file data during transfer. Useful for network transfers or slow connections:

```bash
rsync -avz source/ dest/
```

Default compression algorithm is zlib.

#### `--compress-choice=ALGORITHM`

Choose compression algorithm. Options: `zstd`, `lz4`, `zlib`

```bash
# High compression ratio (slower)
rsync -av --compress-choice=zstd source/ dest/

# Fast compression (lower ratio)
rsync -av --compress-choice=lz4 source/ dest/

# Balanced (default)
rsync -av --compress-choice=zlib source/ dest/
```

**Comparison:**
- **zstd**: Best compression ratio, moderate speed
- **lz4**: Fastest, lower compression ratio
- **zlib**: Balanced (default)

#### `-W, --whole-file`

Copy files whole (no delta-transfer algorithm). Faster for local transfers where files are very different:

```bash
rsync -aW source/ dest/
```

**When to use:**
- Local transfers on fast storage
- When files are completely new or have changed entirely
- When CPU is slower than disk I/O

#### `--inplace`

Update destination files in-place instead of creating a temporary file:

```bash
rsync -a --inplace source/ dest/
```

**Advantages:**
- Uses less disk space (no temporary files)
- Useful when disk space is limited

**Disadvantages:**
- If transfer is interrupted, destination file may be corrupted
- Cannot preserve hard links

#### `--partial`

Keep partially transferred files:

```bash
rsync -a --partial source/ dest/
```

Useful for resuming interrupted transfers.

#### `--partial-dir=DIR`

Put partial files into specified directory:

```bash
rsync -a --partial-dir=.rsync-partial source/ dest/
```

Keeps partial files out of the way during transfer.

#### `--bwlimit=RATE`

Limit I/O bandwidth to RATE KBytes per second:

```bash
# Limit to 1000 KB/s (1 MB/s)
rsync -av --bwlimit=1000 source/ dest/
```

Useful for:
- Limiting network bandwidth usage
- Preventing rsync from saturating your connection
- Background transfers that shouldn't impact other activities

### Delete Options

#### `--delete`

Delete extraneous files from destination directories:

```bash
rsync -av --delete source/ dest/
```

Files that exist in destination but not in source will be deleted.

**Warning:** Use with caution! Always test with `--dry-run` first:

```bash
rsync -avn --delete source/ dest/
```

#### `--delete-before`

Receiver deletes before transfer (not during):

```bash
rsync -av --delete-before source/ dest/
```

**Use case:** Frees up space before transferring new files.

#### `--delete-during`

Receiver deletes during the transfer (default):

```bash
rsync -av --delete-during source/ dest/
```

#### `--delete-after`

Receiver deletes after the transfer:

```bash
rsync -av --delete-after source/ dest/
```

**Use case:** Safer, as files are only deleted after successful transfer.

#### `--delete-excluded`

Also delete excluded files from destination:

```bash
rsync -av --delete --delete-excluded --exclude='*.tmp' source/ dest/
```

Without this option, excluded files in destination are left alone.

#### `--remove-source-files`

Sender removes synchronized files (non-directories):

```bash
rsync -av --remove-source-files source/ dest/
```

**Use case:** Move files instead of copying. Source directories remain but are empty.

**Warning:** Files are deleted from source after successful transfer!

### Filtering Options

#### `--exclude=PATTERN`

Exclude files matching PATTERN:

```bash
# Exclude single pattern
rsync -av --exclude='*.tmp' source/ dest/

# Multiple excludes
rsync -av --exclude='*.tmp' --exclude='*.log' --exclude='.git' source/ dest/
```

**Pattern syntax:**
- `*.tmp`: All files ending in .tmp
- `temp*`: All files starting with temp
- `dir/`: All files in directory named "dir"
- `/absolute`: Pattern from root of transfer

#### `--exclude-from=FILE`

Read exclude patterns from FILE:

```bash
rsync -av --exclude-from=exclude-list.txt source/ dest/
```

Example `exclude-list.txt`:
```
*.tmp
*.log
.git
node_modules
__pycache__
*.pyc
Thumbs.db
.DS_Store
```

**Format:**
- One pattern per line
- Lines starting with `#` are comments
- Blank lines are ignored

#### `--include=PATTERN`

Don't exclude files matching PATTERN:

```bash
# Include only .txt files, exclude everything else
rsync -av --include='*.txt' --exclude='*' source/ dest/
```

**Order matters!** Include patterns are processed before exclude patterns in the order specified.

#### `--include-from=FILE`

Read include patterns from FILE:

```bash
rsync -av --include-from=include-list.txt --exclude='*' source/ dest/
```

#### `--files-from=FILE`

Read list of source files from FILE:

```bash
rsync -av --files-from=file-list.txt source/ dest/
```

Example `file-list.txt`:
```
important/file1.txt
important/file2.txt
data/report.pdf
```

**Use case:** Transfer only specific files from a large directory tree.

### Filtering Examples

#### Example 1: Exclude Multiple File Types

```bash
rsync -av \
  --exclude='*.tmp' \
  --exclude='*.log' \
  --exclude='*.bak' \
  source/ dest/
```

#### Example 2: Include Only Specific File Types

```bash
# Include only .jpg, .png, .gif, exclude everything else
rsync -av \
  --include='*.jpg' \
  --include='*.png' \
  --include='*.gif' \
  --include='*/' \
  --exclude='*' \
  source/ dest/
```

Note: `--include='*/'` is needed to recurse into directories.

#### Example 3: Complex Filter

```bash
# Include specific directories and file types, exclude everything else
rsync -av \
  --include='/important/' \
  --include='/important/**' \
  --include='*.doc' \
  --include='*.pdf' \
  --exclude='*' \
  source/ dest/
```

### Output Options

#### `--progress`

Show progress during transfer:

```bash
rsync -av --progress source/ dest/
```

Output shows:
- Current file being transferred
- Bytes transferred / total bytes
- Transfer speed
- Time remaining (estimated)

#### `-i, --itemize-changes`

Output a change-summary for all updates:

```bash
rsync -avi source/ dest/
```

**Output format:**
```
YXcst...... path\to\file
```

Where:
- **Y** (update type):
  - `>`: File is being sent to the destination
  - `<`: File is being received from remote
  - `c`: Local change/creation
  - `.`: Item is not being updated
  - `*`: Message (e.g., "deleting")
- **X** (file type):
  - `f`: File
  - `d`: Directory
  - `L`: Symlink
  - `D`: Device
  - `S`: Special file
- **c**: Checksum differs
- **s**: Size differs
- **t**: Modification time differs
- **p**: Permissions differ (always `.` on Windows)
- **o**: Owner differs (always `.` on Windows)
- **g**: Group differs (always `.` on Windows)

**Example output:**
```
>f+++++++++ new_file.txt          # New file being sent
.f..t...... existing_file.txt     # File exists, time differs
cd+++++++++ new_dir/               # New directory created
*deleting   old_file.txt           # File being deleted
```

#### `--stats`

Give some file-transfer stats:

```bash
rsync -av --stats source/ dest/
```

**Output includes:**
- Number of files transferred
- Total file size
- Total transferred file size (after delta-transfer)
- Literal data
- Matched data
- File list size
- Total bytes sent/received
- Speedup achieved

Example output:
```
Number of files: 1,234
Number of files transferred: 56
Total file size: 1.23 GB
Total transferred file size: 234 MB
Literal data: 12 MB
Matched data: 222 MB
File list size: 45 KB
Total bytes sent: 12.5 MB
Total bytes received: 234 KB

sent 12.5 MB  received 234 KB  12.7 MB/sec
total size is 1.23 GB  speedup is 96.85
```

#### `-h, --human-readable`

Output numbers in a human-readable format:

```bash
rsync -avh --progress source/ dest/
```

Shows sizes as `1.2M`, `3.4G`, etc. instead of byte counts.

#### `--log-file=FILE`

Log operations to specified FILE:

```bash
rsync -av --log-file=rsync.log source/ dest/
```

Useful for:
- Auditing transfers
- Debugging issues
- Automated backups (review logs later)

### Backup Options

#### `-b, --backup`

Make backups of existing files before overwriting:

```bash
rsync -avb source/ dest/
```

By default, adds `~` suffix to backup files.

#### `--backup-dir=DIR`

Store backups in specified directory with hierarchy preserved:

```bash
rsync -av --backup --backup-dir=../backup source/ dest/
```

Original directory structure is maintained in backup directory.

#### `--suffix=SUFFIX`

Set backup suffix (default: `~`):

```bash
rsync -av --backup --suffix=.bak source/ dest/
```

Creates backups like `file.txt.bak`.

**Example backup workflow:**
```bash
# Create timestamped backups
BACKUP_DIR="backups/$(date +%Y%m%d-%H%M%S)"
rsync -av --backup --backup-dir="$BACKUP_DIR" source/ dest/
```

### Control Options

#### `-n, --dry-run`

Perform a trial run with no changes made:

```bash
rsync -avn --delete source/ dest/
```

**Essential for:**
- Testing rsync commands before running them for real
- Verifying what files will be transferred or deleted
- Checking filter patterns

**Always use --dry-run first** when using `--delete` or other destructive options!

#### `--list-only`

List files instead of copying them:

```bash
rsync -a --list-only source/
```

Shows what would be transferred without actually transferring.

#### `--size-only`

Skip files that match in size:

```bash
rsync -av --size-only source/ dest/
```

Ignores modification time, only compares file sizes.

**Use case:** When timestamps are unreliable but file sizes are accurate.

#### `--timeout=SECONDS`

Set I/O timeout in seconds:

```bash
rsync -av --timeout=300 source/ dest/
```

Useful for network transfers that might hang.

### Checksum Options

#### `--checksum-choice=ALGORITHM`

Choose checksum algorithm. Options: `md4`, `md5`, `blake2`, `xxh128`

```bash
# Use MD5 (default)
rsync -ac --checksum-choice=md5 source/ dest/

# Use Blake2 (modern, secure)
rsync -ac --checksum-choice=blake2 source/ dest/

# Use XXH128 (fastest)
rsync -ac --checksum-choice=xxh128 source/ dest/
```

**Comparison:**
- **md5**: Standard, good balance (default)
- **md4**: Legacy, faster but less secure
- **blake2**: Modern, cryptographically secure
- **xxh128**: Fastest, non-cryptographic

### Remote Transfer Options

#### `-e, --rsh=COMMAND`

Specify the remote shell program to use:

```bash
rsync -av -e "ssh -i /path/to/key" source/ user@host:dest/
```

Commonly used with SSH for remote transfers.

**SSH Authentication:**
YARW supports multiple SSH authentication methods:
1. **SSH Agent** (default): Uses your SSH agent for authentication
2. **Public Key**: Specify with `-e "ssh -i /path/to/key"`
3. **Password**: If agent and public key fail, prompts for password

**Example with SSH options:**
```bash
rsync -av -e "ssh -p 2222 -i ~/.ssh/id_rsa" source/ user@host:dest/
```

#### `--rsync-path=PATH`

Specify the path to rsync on the remote machine:

```bash
rsync -av --rsync-path=/usr/local/bin/rsync source/ user@host:dest/
```

Useful when rsync is not in the default PATH on the remote system.

### Daemon Mode Options

#### `--daemon`

Run as an rsync daemon:

```bash
rsync --daemon
```

Starts rsync in daemon mode, listening for incoming connections.

#### `--address=ADDRESS`

Bind to the specified address when running in daemon mode:

```bash
rsync --daemon --address=192.168.1.100
```

#### `--port=PORT`

Specify the TCP port for daemon mode (default: 873):

```bash
rsync --daemon --port=8873
```

#### `--config=FILE`

Specify an alternate config file for daemon mode:

```bash
rsync --daemon --config=/etc/rsyncd.conf
```

Default is `rsyncd.conf` in the current directory.

#### `--password-file=FILE`

Read daemon password from FILE:

```bash
rsync -av --password-file=rsync.pwd rsync://user@host/module/
```

The file should contain only the password.

## Advanced Usage

### Combining Options for Common Scenarios

#### Scenario 1: Complete Backup with Verification

```bash
rsync -avc --delete --backup --backup-dir=../backup_$(date +%Y%m%d) source/ dest/
```

- `-a`: Archive mode
- `-v`: Verbose
- `-c`: Checksum verification
- `--delete`: Remove files not in source
- `--backup --backup-dir`: Save overwritten files

#### Scenario 2: Bandwidth-Limited Network Transfer

```bash
rsync -avz --compress-choice=zstd --bwlimit=5000 --progress source/ dest/
```

- `-z --compress-choice=zstd`: High compression
- `--bwlimit=5000`: Limit to 5 MB/s
- `--progress`: Monitor transfer

#### Scenario 3: Selective Sync with Filters

```bash
rsync -av \
  --include='*.doc' \
  --include='*.pdf' \
  --include='*.xlsx' \
  --include='*/' \
  --exclude='*' \
  --progress \
  source/ dest/
```

Syncs only Office documents.

#### Scenario 4: Incremental Backup

```bash
rsync -avu --itemize-changes --stats source/ dest/
```

- `-u`: Only update newer files
- `--itemize-changes`: Show what's being updated
- `--stats`: Summary at end

### Multiple Sources

You can specify multiple source paths:

```bash
rsync -av source1/ source2/ source3/ destination/
```

Each source is processed in order.

### Windows-Specific Usage

#### UNC Paths

```bash
rsync -av C:\Data\ \\server\share\Backup\
```

#### Long Paths

Long paths (>260 characters) are automatically handled:

```bash
rsync -av "C:\Very\Long\Path\That\Exceeds\260\Characters\..." D:\Backup\
```

#### Cross-Drive Sync

```bash
rsync -av C:\Data\ X:\Backup\
```

## Performance Tuning

### For Local Transfers

**Fast storage (SSD):**
```bash
rsync -aW source/ dest/
```

Use `--whole-file` to skip delta calculations.

**Large files with small changes:**
```bash
rsync -av source/ dest/
```

Default delta-transfer is optimal.

### For Network Transfers

**Fast network:**
```bash
rsync -av --compress-choice=lz4 source/ dest/
```

Use light compression to minimize CPU overhead.

**Slow network:**
```bash
rsync -avz --compress-choice=zstd source/ dest/
```

Use high compression to minimize data transfer.

### For Many Small Files

```bash
rsync -av --size-only source/ dest/
```

Skip time comparisons to speed up scanning.

### For Large Files

```bash
rsync -av --partial --inplace source/ dest/
```

Enable partial transfers and in-place updates for resumability.

## Troubleshooting

### Common Issues

#### Issue 1: "File not found" errors

**Solution:** Check path quoting:

```bash
# Wrong (spaces cause issues)
rsync -av C:\My Documents\ D:\Backup\

# Correct
rsync -av "C:\My Documents\" "D:\Backup\"
```

#### Issue 2: Too many files deleted

**Solution:** Always use `--dry-run` first:

```bash
# Test first
rsync -avn --delete source/ dest/

# If output looks correct, run for real
rsync -av --delete source/ dest/
```

#### Issue 3: Slow performance

**Solution:** Try different options:

```bash
# Skip delta-transfer for local copies
rsync -aW source/ dest/

# Use checksums only when needed
rsync -av source/ dest/   # Default uses size/time

# Limit verbosity
rsync -aq source/ dest/   # Quiet mode
```

#### Issue 4: Insufficient space for temporary files

**Solution:** Use `--inplace`:

```bash
rsync -av --inplace source/ dest/
```

#### Issue 5: Transfer interrupted

**Solution:** Resume with `--partial`:

```bash
rsync -av --partial source/ dest/
```

### Getting Help

View built-in help:

```bash
rsync --help
```

For this documentation:

```bash
# View README
type README.md

# View this usage guide
type USAGE.md
```

## Summary of Most Useful Option Combinations

```bash
# Standard sync
rsync -av source/ dest/

# Backup with delete
rsync -av --delete source/ dest/

# Dry run before real operation
rsync -avn --delete source/ dest/

# Detailed progress
rsync -avi --progress --stats source/ dest/

# Compressed transfer
rsync -avz source/ dest/

# Incremental backup
rsync -avu source/ dest/

# Mirror with verification
rsync -avc --delete source/ dest/

# Network transfer with bandwidth limit
rsync -avz --bwlimit=1000 source/ dest/

# Filtered sync
rsync -av --exclude='*.tmp' --exclude='*.log' source/ dest/

# Safe backup with old file preservation
rsync -av --backup --backup-dir=../old source/ dest/
```

## Next Steps

- See [EXAMPLES.md](EXAMPLES.md) for real-world usage examples
- See [README.md](../README.md) for project overview
