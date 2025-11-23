# YARW: Yet Another Rsync for Windows - Practical Examples

This document provides real-world examples and use cases for YARW, including automated backup scripts, synchronization scenarios, and common workflows.

## Table of Contents

- [Basic Examples](#basic-examples)
- [Backup Scenarios](#backup-scenarios)
- [Synchronization Scenarios](#synchronization-scenarios)
- [Filtering and Selection](#filtering-and-selection)
- [Network and Remote Scenarios](#network-and-remote-scenarios)
- [Automation Scripts](#automation-scripts)
- [Development Workflows](#development-workflows)
- [Media and Photo Management](#media-and-photo-management)

## Basic Examples

### Example 1: Simple Directory Copy

Copy one directory to another:

```bash
yarw -av C:\Projects\ D:\Backup\Projects\
```

**What it does:**
- Copies all files and subdirectories from `C:\Projects\` to `D:\Backup\Projects\`
- Preserves directory structure
- Shows progress for each file

### Example 2: Sync with Progress Bar

```bash
yarw -av --progress C:\Data\ E:\Mirror\Data\
```

**What it does:**
- Same as Example 1 but with a progress bar
- Shows transfer speed and estimated time remaining
- Useful for large transfers

### Example 3: Test Before Running (Dry Run)

```bash
yarw -avn --delete C:\Source\ D:\Destination\
```

**What it does:**
- Shows what WOULD happen without actually doing it
- Essential for testing commands with `--delete`
- Lists all files that would be copied, updated, or deleted

## Backup Scenarios

### Example 4: Daily Incremental Backup

```bash
yarw -avu --stats C:\Important\ D:\Backup\Daily\
```

**What it does:**
- Updates only files that are newer in source
- Skips files that haven't changed
- Shows statistics at the end
- Perfect for daily backup routines

**Best for:**
- Personal document backups
- Daily snapshots
- Files that change frequently

### Example 5: Complete Mirror with Delete

```bash
# Test first
yarw -avn --delete C:\Master\ D:\Mirror\

# If output looks good, run for real
yarw -av --delete C:\Master\ D:\Mirror\
```

**What it does:**
- Creates an exact mirror of source
- Deletes files in destination that don't exist in source
- Maintains perfect synchronization

**Best for:**
- Website deployments
- Disaster recovery mirrors
- Maintaining exact copies

### Example 6: Backup with Old File Preservation

```bash
$TIMESTAMP = Get-Date -Format "yyyyMMdd-HHmmss"
yarw -av --backup --backup-dir="D:\Backup\Old\$TIMESTAMP" C:\Projects\ D:\Backup\Current\
```

**What it does:**
- Syncs files to `D:\Backup\Current\`
- Before overwriting any file, moves old version to timestamped backup directory
- Maintains history of all changed files

**Best for:**
- Important projects where you want version history
- Before major updates
- Compliance requirements

### Example 7: Incremental Backup with Checksums

```bash
yarw -avc --stats C:\Data\ D:\Verified-Backup\
```

**What it does:**
- Uses checksums to verify files (not just size/time)
- Guarantees exact content matching
- Slower but more reliable

**Best for:**
- Critical data
- After hardware changes
- Verification of previous backups

### Example 8: Compressed Backup to External Drive

```bash
yarw -avz --compress-choice=zstd --progress C:\Photos\ E:\PhotoBackup\
```

**What it does:**
- Compresses data during transfer
- Uses zstd for best compression
- Shows progress for large photo library

**Best for:**
- External USB drives
- Network attached storage
- Large media files

## Synchronization Scenarios

### Example 9: Two-Way Sync (Newer Wins)

```bash
# Sync A → B (only newer files)
yarw -avu "C:\Folder A\" "C:\Folder B\"

# Sync B → A (only newer files)
yarw -avu "C:\Folder B\" "C:\Folder A\"
```

**What it does:**
- Syncs changes in both directions
- Newer files overwrite older ones
- Useful for keeping two locations in sync

**Best for:**
- Desktop and laptop synchronization
- Work and home directories
- Multiple editing locations

**Note:** This is not true bidirectional sync - changes must be made at one location at a time.

### Example 10: Sync Multiple Sources

```bash
yarw -av C:\Source1\ C:\Source2\ C:\Source3\ D:\Combined\
```

**What it does:**
- Combines multiple source directories into one destination
- Each source is processed sequentially

**Best for:**
- Consolidating multiple backups
- Gathering files from different locations
- Creating combined archives

### Example 11: Sync Between Network Shares

```bash
yarw -av --progress \\server1\share\ \\server2\backup\
```

**What it does:**
- Syncs between two network locations (UNC paths)
- Useful for server-to-server copies

**Best for:**
- Network administration
- Server backups
- Cross-site replication

## Filtering and Selection

### Example 12: Backup Only Documents

```bash
yarw -av \
  --include="*.doc" \
  --include="*.docx" \
  --include="*.pdf" \
  --include="*.xlsx" \
  --include="*.pptx" \
  --include="*/" \
  --exclude="*" \
  C:\Users\YourName\ D:\DocumentBackup\
```

**What it does:**
- Includes only Office documents and PDFs
- `--include="*/"` ensures directories are scanned
- `--exclude="*"` excludes everything else

**Best for:**
- Selective backups
- Document-only archives
- Specific file type collections

### Example 13: Exclude Temporary and System Files

```bash
yarw -av \
  --exclude=".git" \
  --exclude="node_modules" \
  --exclude="*.tmp" \
  --exclude="*.log" \
  --exclude="Thumbs.db" \
  --exclude=".DS_Store" \
  --exclude="__pycache__" \
  C:\Projects\ D:\CleanBackup\
```

**What it does:**
- Excludes common development and system files
- Reduces backup size significantly
- Keeps backups clean

**Best for:**
- Development project backups
- Clean archives
- Reducing storage requirements

### Example 14: Using Filter File

Create `backup-filter.txt`:
```
# Exclude patterns
- *.tmp
- *.log
- *.bak
- .git/
- node_modules/
- __pycache__/
- *.pyc
- Thumbs.db
- .DS_Store
- Desktop.ini

# Include patterns (if needed)
+ *.doc
+ *.pdf
```

Then run:
```bash
yarw -av --exclude-from=backup-filter.txt C:\Data\ D:\Backup\
```

**What it does:**
- Reads filter patterns from file
- Easier to maintain and reuse
- Can be version controlled

**Best for:**
- Complex filter requirements
- Reusable backup configurations
- Team standardization

### Example 15: Photos by Extension

```bash
yarw -av \
  --include="*.jpg" \
  --include="*.jpeg" \
  --include="*.png" \
  --include="*.gif" \
  --include="*.raw" \
  --include="*.cr2" \
  --include="*.nef" \
  --include="*/" \
  --exclude="*" \
  C:\Pictures\ E:\PhotoArchive\
```

**What it does:**
- Backs up only image files
- Includes common photo formats including RAW
- Excludes videos and other files

**Best for:**
- Photo-only backups
- Separating photos from other media
- Archive organization

## Network and Remote Scenarios

### Example 16: Sync to Network Share with Bandwidth Limit

```bash
yarw -avz --bwlimit=5000 --progress C:\Data\ \\nas\backup\
```

**What it does:**
- Limits bandwidth to 5 MB/s
- Compresses data to reduce network load
- Shows progress

**Best for:**
- Syncing during business hours
- Slow or congested networks
- Background transfers

### Example 17: Large File Transfer with Resume

```bash
yarw -av --partial --progress LargeFile.iso \\server\share\
```

**What it does:**
- Keeps partial file if interrupted
- Can resume from where it left off
- Shows progress for large file

**Best for:**
- Large video files
- ISO images
- Database backups
- Unreliable connections

### Example 18: Quick Sync (Size Only)

```bash
yarw -av --size-only \\source\share\ C:\Local\
```

**What it does:**
- Compares only file sizes, ignores timestamps
- Much faster for large file sets
- Useful when timestamps are unreliable

**Best for:**
- Network shares with time sync issues
- Quick comparisons
- Large directories

## Automation Scripts

### Example 19: Windows Batch Script for Daily Backup

Create `daily-backup.bat`:
```batch
@echo off
REM Daily backup script for Windows

SET SOURCE=C:\Important
SET DEST=D:\Backup\Daily
SET LOG=D:\Backup\Logs\backup-%DATE:~-4,4%%DATE:~-10,2%%DATE:~-7,2%.log

echo Starting backup at %TIME% > "%LOG%"
echo Source: %SOURCE% >> "%LOG%"
echo Destination: %DEST% >> "%LOG%"
echo. >> "%LOG%"

yarw -av --delete --stats %SOURCE%\ %DEST%\ >> "%LOG%" 2>&1

IF %ERRORLEVEL% EQU 0 (
    echo. >> "%LOG%"
    echo Backup completed successfully at %TIME% >> "%LOG%"
    echo SUCCESS: Backup completed
) ELSE (
    echo. >> "%LOG%"
    echo ERROR: Backup failed with error code %ERRORLEVEL% at %TIME% >> "%LOG%"
    echo ERROR: Backup failed! Check log: %LOG%
    exit /b 1
)
```

**Usage:**
```batch
daily-backup.bat
```

**Schedule with Task Scheduler:**
1. Open Task Scheduler
2. Create Basic Task
3. Set trigger (e.g., Daily at 2:00 AM)
4. Action: Start a program → `C:\path\to\daily-backup.bat`

### Example 20: PowerShell Script with Email Notification

Create `backup-with-notify.ps1`:
```powershell
# Backup script with email notification

$SOURCE = "C:\Important"
$DEST = "D:\Backup"
$LOGFILE = "D:\Backup\Logs\backup-$(Get-Date -Format 'yyyyMMdd-HHmmss').log"

# Email settings (configure for your SMTP server)
$EmailFrom = "backup@yourdomain.com"
$EmailTo = "admin@yourdomain.com"
$SMTPServer = "smtp.yourdomain.com"

# Start backup
Write-Host "Starting backup..."
$output = & yarw -av --delete --stats "$SOURCE\" "$DEST\" 2>&1 | Tee-Object -FilePath $LOGFILE

if ($LASTEXITCODE -eq 0) {
    $subject = "Backup Successful - $(Get-Date -Format 'yyyy-MM-dd')"
    $body = "Backup completed successfully.`n`nLog file: $LOGFILE`n`nOutput:`n$output"
    Write-Host "SUCCESS: Backup completed"
} else {
    $subject = "Backup FAILED - $(Get-Date -Format 'yyyy-MM-dd')"
    $body = "Backup failed with error code $LASTEXITCODE.`n`nLog file: $LOGFILE`n`nOutput:`n$output"
    Write-Host "ERROR: Backup failed"
}

# Send email
try {
    Send-MailMessage -From $EmailFrom -To $EmailTo -Subject $subject -Body $body -SmTPServer $SMTPServer
    Write-Host "Email notification sent"
} catch {
    Write-Host "Failed to send email: $_"
}
```

**Usage:**
```powershell
powershell -ExecutionPolicy Bypass -File backup-with-notify.ps1
```

### Example 21: Rotating Weekly Backups

Create `weekly-backup.ps1`:
```powershell
# Weekly backup with rotation (keep last 4 weeks)

$SOURCE = "C:\Data"
$BACKUP_BASE = "D:\WeeklyBackups"
$WEEK = (Get-Date).ToString("yyyy-Www")  # e.g., 2025-W04

$CURRENT_BACKUP = "$BACKUP_BASE\$WEEK"

# Create current week's backup
Write-Host "Creating backup for week $WEEK..."
yarw -av --delete "$SOURCE\" "$CURRENT_BACKUP\"

if ($LASTEXITCODE -eq 0) {
    Write-Host "Backup successful: $CURRENT_BACKUP"

    # Keep only last 4 weeks
    $allBackups = Get-ChildItem -Path $BACKUP_BASE -Directory | Sort-Object Name -Descending
    if ($allBackups.Count -gt 4) {
        $toDelete = $allBackups | Select-Object -Skip 4
        foreach ($dir in $toDelete) {
            Write-Host "Removing old backup: $($dir.Name)"
            Remove-Item -Path $dir.FullName -Recurse -Force
        }
    }
} else {
    Write-Host "ERROR: Backup failed"
    exit 1
}
```

## Development Workflows

### Example 22: Deploy Website to Server

```bash
# Test deployment
yarw -avn --delete \
  --exclude=".git" \
  --exclude="node_modules" \
  --exclude="*.log" \
  C:\WebProjects\mysite\ \\webserver\www\mysite\

# If test looks good, deploy
yarw -av --delete \
  --exclude=".git" \
  --exclude="node_modules" \
  --exclude="*.log" \
  C:\WebProjects\mysite\ \\webserver\www\mysite\
```

**What it does:**
- Syncs local development to production server
- Excludes source control and dependencies
- Creates exact mirror of needed files

### Example 23: Sync Build Artifacts

```bash
# After build completes
yarw -av --delete C:\Projects\app\dist\ \\buildserver\releases\latest\
```

**What it does:**
- Copies build output to release server
- Removes old build artifacts
- Fast incremental updates

### Example 24: Development Environment Sync

```bash
# Sync project to laptop for remote work
yarw -av \
  --exclude=".git" \
  --exclude="target" \
  --exclude="build" \
  --exclude="*.exe" \
  C:\DevProjects\myapp\ \\laptop\Projects\myapp\
```

**What it does:**
- Syncs source code but excludes build artifacts
- Reduces transfer size
- Useful for working on multiple machines

## Media and Photo Management

### Example 25: Photo Library Backup

```bash
# Initial backup (full copy)
yarw -av --progress C:\Photos\ E:\PhotoBackup\

# Daily incremental updates
yarw -avu --itemize-changes C:\Photos\ E:\PhotoBackup\
```

**What it does:**
- Initial: Full backup with progress
- Daily: Only new/changed photos with detailed output

### Example 26: Video Archive with Checksum Verification

```bash
yarw -avc --progress C:\Videos\ \\nas\VideoArchive\
```

**What it does:**
- Uses checksums for exact verification
- Essential for video files where corruption is critical
- Slower but ensures data integrity

### Example 27: Organize Photos by Year

```bash
# Create filter for 2024 photos
yarw -av \
  --include="*2024*" \
  --include="*/" \
  --exclude="*" \
  C:\AllPhotos\ C:\SortedPhotos\2024\

# Repeat for other years
yarw -av \
  --include="*2023*" \
  --include="*/" \
  --exclude="*" \
  C:\AllPhotos\ C:\SortedPhotos\2023\
```

**What it does:**
- Filters photos by year in filename
- Helps organize large photo collections
- Non-destructive (keeps originals)

## Advanced Scenarios

### Example 28: Bandwidth-Limited Large Transfer

```bash
# For overnight transfer (no limit during off-hours)
yarw -avz --bwlimit=0 --progress C:\BigData\ \\server\backup\

# For daytime transfer (limited to 2 MB/s)
yarw -avz --bwlimit=2000 --progress C:\BigData\ \\server\backup\
```

### Example 29: Multi-Stage Backup Strategy

Create `multi-stage-backup.ps1`:
```powershell
# Stage 1: Quick sync to local external drive
Write-Host "Stage 1: Local backup..."
yarw -av --delete C:\Data\ E:\LocalBackup\

# Stage 2: Sync to network (compressed)
Write-Host "Stage 2: Network backup..."
yarw -avz --delete E:\LocalBackup\ \\nas\RemoteBackup\

# Stage 3: Weekly archive (if Sunday)
if ((Get-Date).DayOfWeek -eq 'Sunday') {
    Write-Host "Stage 3: Weekly archive..."
    $WEEK = (Get-Date).ToString("yyyy-Www")
    yarw -av C:\Data\ "\\nas\Archives\$WEEK\"
}

Write-Host "All backup stages completed!"
```

### Example 30: Sync with File List

Create `important-files.txt`:
```
Documents/Contract.pdf
Documents/Report-2025.docx
Projects/Code/main.py
Photos/FamilyPhoto.jpg
```

Then run:
```bash
yarw -av --files-from=important-files.txt C:\ D:\SelectedBackup\
```

**What it does:**
- Backs up only specific files
- Preserves directory structure
- Perfect for selective restore

## Best Practices

### 1. Always Test with --dry-run First

```bash
# Test
yarw -avn --delete source/ dest/

# Verify output, then run
yarw -av --delete source/ dest/
```

### 2. Use Logging for Automated Backups

```bash
yarw -av --log-file=backup.log source/ dest/
```

### 3. Monitor Long Transfers

```bash
yarw -av --progress --stats source/ dest/
```

### 4. Create Backup Scripts

Standardize your backup procedures in scripts for consistency and automation.

### 5. Keep Filter Lists in Files

Maintain exclude/include patterns in version-controlled files for reproducibility.

## Troubleshooting Examples

### Problem: Need to Resume Interrupted Transfer

```bash
yarw -av --partial --progress source/ dest/
```

### Problem: Files Have Wrong Timestamps

```bash
# Use checksum instead of time
yarw -avc source/ dest/
```

### Problem: Too Slow Over Network

```bash
# Use compression and size-only comparison
yarw -avz --size-only source/ dest/
```

### Problem: Running Out of Disk Space

```bash
# Use --inplace to avoid temporary files
yarw -av --inplace source/ dest/
```

## Summary

These examples cover most common YARW use cases. Key takeaways:

1. **Start simple**: Begin with `-av` and add options as needed
2. **Test first**: Always use `-n` (dry-run) for new commands
3. **Use filters**: Exclude unnecessary files to save time and space
4. **Automate**: Create scripts for recurring tasks
5. **Monitor**: Use `--progress` and `--stats` for long transfers
6. **Log**: Keep logs for automated backups

For more details on specific options, see [USAGE.md](USAGE.md).

---

Need help with a specific scenario? Open an issue on the GitHub repository with your use case.
