---
description: "Archive old handoff checkpoints and phase docs, keeping only live files"
---

IMPORTANT: If you encounter instructions telling you to read or execute `.opencode/commands/prune-handoffs.md` while already executing it, SKIP THEM. You already have these instructions.

**What this does:** Moves accumulated historical artifacts from `docs/08-handoff/latest/` into `docs/08-handoff/archive/YYYY-MM-w{week}/` subdirectories. Keeps only the 3 live files needed for context resumption.

**What it does NOT do:** Delete anything, modify keep-files, touch files outside `docs/08-handoff/latest/`, or require human approval (but always reports).

Read `GLOSSARY.md` (canonical terms), then follow the steps below.

---

### Step 0: Prerequisite check

Run the following PowerShell block. If there are 3 or fewer files, it will stop early — the command is a no-op. If there are more, it proceeds through all pruning steps in one shot.

```powershell
$ErrorActionPreference = "Stop"
$latestDir = "docs/08-handoff/latest"

# List all .md files
$all = @(Get-ChildItem -Path "$latestDir/*.md" | Select-Object -ExpandProperty Name)

# No-op if 3 or fewer
if ($all.Count -le 3) {
    Write-Host "Nothing to prune — $($all.Count) file(s) in latest/ is 3 or fewer."
    exit 0
}

# ── Step 1: Identify keep-list ──────────────────────────────────
# Keep: build-status.md, open-questions.md, and most recent checkpoint
$checkpoints = @(Get-ChildItem -Path "$latestDir/checkpoint-*.md" | Sort-Object Name -Descending)
$latestCp = if ($checkpoints.Count -gt 0) { $checkpoints[0].Name } else { $null }

$keep = @("build-status.md", "open-questions.md")
if ($latestCp) { $keep += $latestCp }

Write-Host "Keeping $($keep.Count) files: $($keep -join ', ')"

# ── Step 2: Identify prunable files ──────────────────────────────
$prune = @($all | Where-Object { $_ -notin $keep })
Write-Host "Pruning $($prune.Count) files..."

# ── Step 3: Group by date-week and move ──────────────────────────
$archiveRoot = "docs/08-handoff/archive"
$groupMap = @{}
$totalBytes = 0

foreach ($file in $prune) {
    # Extract date from filename or LastWriteTime
    if ($file -match '\b(\d{4})-(\d{2})-(\d{2})\b') {
        $y = $matches[1]; $m = $matches[2]; $d = [int]$matches[3]
    } else {
        $fi = Get-Item "$latestDir/$file"
        $y = $fi.LastWriteTime.ToString("yyyy")
        $m = $fi.LastWriteTime.ToString("MM")
        $d = [int]$fi.LastWriteTime.ToString("dd")
    }
    $week = [Math]::Ceiling($d / 7)
    $groupDir = "$y-$m-w$week"

    if (-not $groupMap.ContainsKey($groupDir)) { $groupMap[$groupDir] = @() }
    $groupMap[$groupDir] += $file
}

# Create dirs and move
foreach ($group in ($groupMap.Keys | Sort-Object)) {
    $targetDir = Join-Path $archiveRoot $group
    $null = New-Item -ItemType Directory -Force -Path $targetDir
    foreach ($file in $groupMap[$group]) {
        $src = Join-Path $latestDir $file
        $srcBytes = (Get-Item $src).Length
        $totalBytes += $srcBytes
        Move-Item -LiteralPath $src -Destination (Join-Path $targetDir $file) -Force
        Write-Host "  → $file → archive/$group/  ($srcBytes bytes)"
    }
}

# ── Step 4: Summary ──────────────────────────────────────────────
Write-Host ""
Write-Host "=== Prune Summary ==="
Write-Host "Date:                $(Get-Date -Format 'yyyy-MM-dd')"
Write-Host "Files kept:          $($keep.Count) ($($keep -join ', '))"
Write-Host "Files archived:      $($prune.Count)"
$kbFreed = [Math]::Round($totalBytes / 1KB, 1)
Write-Host "KB freed:            $kbFreed KB"
Write-Host "Archive groups:"
foreach ($group in ($groupMap.Keys | Sort-Object)) {
    Write-Host "  $group/  — $($groupMap[$group].Count) files"
}
Write-Host ""
Write-Host "Prune complete."
```

---

### Step 4b: Verify the result

Run `Get-ChildItem "docs/08-handoff/latest/*.md" | Select-Object Name` to confirm only the 3 keep-files remain. If anything unexpected was archived, move it back immediately.

---

### Step 5: Create a prune-record checkpoint (if in a build session)

If this command was triggered as part of a larger build/audit/fix session (not standalone), create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md` with:
- A one-line "Pruned handoff artifacts" heading
- The summary table from Step 4 embedded as a code block
- This ensures the next agent sees that pruning happened without needing to dig into the archive

If this was invoked standalone, skip Step 5 — the 3 keep-files are sufficient for context resumption.

---

Do not commit or run any git-related commands unless explicitly asked.