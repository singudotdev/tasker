# tasker installer for Windows (PowerShell 5.1+ or PowerShell 7).
#
#   irm <url>/install.ps1 | iex
#
# Options are environment variables, set before running:
#   $env:TASKER_APP         = "gui"         # tui (default): the terminal UI `tasker`; gui: the desktop app
#                                           # `tasker-gui`; both. Both use the same tasks.
#                                           # With UNINSTALL/PURGE the default is both.
#   $env:TASKER_VERSION     = "v0.1.0"      # default: latest
#   $env:TASKER_INSTALL_DIR = "D:\tools"    # default: %LOCALAPPDATA%\Programs\tasker
#   $env:TASKER_UNINSTALL   = "1"           # remove the app(s) (your tasks are kept)
#   $env:TASKER_PURGE       = "1"           # remove the app(s) AND your data: tasks and tags.md
#   $env:TASKER_YES         = "1"           # with PURGE: don't ask for confirmation
# Also: TASKER_REPO, TASKER_HOST (github | gitlab), TASKER_GITLAB_URL, TASKER_BASE_URL.
#
# tasker is installed and used per user: this script refuses to run elevated ("Run as administrator")
# and only installs inside your user profile. (Admin-only accounts: $env:TASKER_ALLOW_ADMIN = "1".)

# Everything lives in a function so `iex` never leaks variables or closes the caller's shell.
function Install-Tasker {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'   # Invoke-WebRequest is very slow with the progress bar

    # ---- where releases are published: edit these two lines for your repository ----
    $repo = if ($env:TASKER_REPO) { $env:TASKER_REPO } else { 'singudotdev/tasker' }
    $hostKind = if ($env:TASKER_HOST) { $env:TASKER_HOST } else { 'github' }

    $version = if ($env:TASKER_VERSION) { $env:TASKER_VERSION } else { 'latest' }
    $installDir = if ($env:TASKER_INSTALL_DIR) { $env:TASKER_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\tasker' }

    function Say($msg) { Write-Host "tasker: $msg" }

    # ---- per-user only ----
    $onWindows = ($PSVersionTable.PSEdition -eq 'Desktop') -or $IsWindows
    if ($onWindows -and $env:TASKER_ALLOW_ADMIN -ne '1') {
        $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
        if ($principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
            throw 'tasker: installed per user: run this in a normal PowerShell, not "Run as administrator". (Admin-only accounts: $env:TASKER_ALLOW_ADMIN = "1")'
        }
    }
    $sep = [IO.Path]::DirectorySeparatorChar
    $userRoot = [IO.Path]::GetFullPath($(if ($env:USERPROFILE) { $env:USERPROFILE } else { $HOME })).TrimEnd('\', '/')
    $installDir = [IO.Path]::GetFullPath($installDir).TrimEnd('\', '/')    # also resolves '..'
    if (-not "$installDir$sep".StartsWith("$userRoot$sep", [StringComparison]::OrdinalIgnoreCase)) {
        throw "tasker: install folder must be inside your user profile ($userRoot), got $installDir"
    }

    # ---- which apps ----
    $purge = $env:TASKER_PURGE -eq '1'
    $uninstall = $purge -or $env:TASKER_UNINSTALL -eq '1'
    $app = if ($env:TASKER_APP) { $env:TASKER_APP } elseif ($uninstall) { 'both' } else { 'tui' }
    $bins = switch ($app) {
        'tui' { @('tasker') }
        'gui' { @('tasker-gui') }
        'both' { @('tasker', 'tasker-gui') }
        default { throw "tasker: TASKER_APP must be tui, gui or both, got $app" }
    }
    # The desktop app's Start Menu entry.
    $shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'tasker.lnk'

    # ---- uninstall ----
    if ($uninstall) {
        # Where tasker keeps its data: ask an installed binary, else mirror its defaults.
        $data = $null
        foreach ($bin in 'tasker', 'tasker-gui') {
            $exe = Join-Path $installDir "$bin.exe"
            if (-not $data -and (Test-Path $exe)) { try { $data = (& $exe path | Select-Object -First 1) } catch { } }
        }
        if (-not $data) { $data = if ($env:TASKER_DIR) { $env:TASKER_DIR } else { Join-Path $env:APPDATA 'tasker' } }

        $owned = @()
        if ($purge) {
            $full = [IO.Path]::GetFullPath($data).TrimEnd('\', '/')
            $unsafe = @([IO.Path]::GetPathRoot($full).TrimEnd('\', '/'), "$env:USERPROFILE".TrimEnd('\'), "$HOME".TrimEnd('\', '/'))
            if (-not $full -or $unsafe -contains $full) { throw "tasker: refusing to delete data in '$data'" }
            if (Test-Path $data -PathType Container) {
                # Only files tasker creates: NNNN-slug.md tasks, tags.md, leftover temp files.
                $owned = @(Get-ChildItem $data -File -Force | Where-Object {
                    $_.Name -match '^\d{4,}-.*\.md$' -or $_.Name -eq 'tags.md' -or $_.Name -match '^\..*\.md\.tmp$' })
                $tasks = @($owned | Where-Object { $_.Name -match '^\d{4,}-.*\.md$' }).Count
                Say "data folder: $data"
                Say "this permanently deletes $tasks task file(s) and tags.md"
                if ($env:TASKER_YES -ne '1') {
                    $answer = ''
                    try { $answer = Read-Host 'tasker: delete your tasker data? [y/N]' } catch {
                        throw 'tasker: no terminal to confirm on; set $env:TASKER_YES = "1" to delete your data'
                    }
                    if ($answer -notmatch '^(y|yes)$') { Say 'cancelled, nothing was removed'; return }
                }
            } else {
                Say "no data folder at $data"
                $data = $null
            }
        }

        foreach ($bin in $bins) {
            $exe = Join-Path $installDir "$bin.exe"
            if (Test-Path $exe) { Remove-Item $exe -Force; Say "removed $exe" } else { Say "no binary at $exe" }
            if ($bin -eq 'tasker-gui' -and (Test-Path $shortcut)) { Remove-Item $shortcut -Force; Say "removed $shortcut" }
        }
        # Once neither app is left, the folder and its PATH entry go too.
        if ((Test-Path $installDir) -and -not (Get-ChildItem $installDir -Force)) { Remove-Item $installDir -Force }
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if ($userPath -and -not (Test-Path $installDir)) {
            $kept = ($userPath -split ';') | Where-Object { $_ -and ($_.TrimEnd('\') -ne $installDir.TrimEnd('\')) }
            [Environment]::SetEnvironmentVariable('Path', ($kept -join ';'), 'User')
        }

        if ($purge -and $data) {
            $owned | Remove-Item -Force
            if (-not (Get-ChildItem $data -Force)) { Remove-Item $data -Force; Say "removed $data" }
            else { Say "deleted tasker's files; kept $data because it contains other files" }
        } elseif (-not $purge) {
            Say "your tasks were kept in $data (set `$env:TASKER_PURGE = `"1`" to delete them too)"
        }
        return
    }

    # ---- detect platform ----
    # Windows on ARM runs the x64 build through emulation.
    $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    if ($arch -notin @('AMD64', 'ARM64')) { throw "tasker: unsupported CPU architecture: $arch (64-bit Windows is required)" }

    # ---- download location ----
    if ($env:TASKER_BASE_URL) {
        $base = $env:TASKER_BASE_URL
    } elseif ($hostKind -eq 'github') {
        $base = if ($version -eq 'latest') { "https://github.com/$repo/releases/latest/download" } else { "https://github.com/$repo/releases/download/$version" }
    } elseif ($hostKind -eq 'gitlab') {
        $gl = if ($env:TASKER_GITLAB_URL) { $env:TASKER_GITLAB_URL } else { 'https://gitlab.com' }
        $base = if ($version -eq 'latest') { "$gl/$repo/-/releases/permalink/latest/downloads" } else { "$gl/$repo/-/releases/$version/downloads" }
    } else {
        throw 'tasker: TASKER_HOST must be github or gitlab'
    }

    # Windows PowerShell 5.1 defaults to old TLS versions.
    try { [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12 } catch { }

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    foreach ($bin in $bins) {
        $asset = "$bin-x86_64-pc-windows-msvc.zip"
        $exe = Join-Path $installDir "$bin.exe"
        $tmp = Join-Path ([IO.Path]::GetTempPath()) ("tasker-" + [Guid]::NewGuid())
        New-Item -ItemType Directory -Path $tmp | Out-Null
        try {
            $zip = Join-Path $tmp $asset
            Say "downloading $asset ($version)"
            try {
                Invoke-WebRequest -Uri "$base/$asset" -OutFile $zip -UseBasicParsing
            } catch {
                throw "tasker: download failed: $base/$asset ($($_.Exception.Message))"
            }

            # ---- verify checksum ----
            $sumFile = Join-Path $tmp "$asset.sha256"
            $haveSum = $true
            try { Invoke-WebRequest -Uri "$base/$asset.sha256" -OutFile $sumFile -UseBasicParsing } catch { $haveSum = $false }
            if ($haveSum) {
                $expected = ((Get-Content $sumFile -Raw).Trim() -split '\s+')[0].ToLower()
                $actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
                if ($actual -ne $expected) { throw "tasker: checksum mismatch for $asset (expected $expected, got $actual)" }
                Say 'checksum ok'
            } else {
                Say "warning: no checksum published for $asset, skipping verification"
            }

            # ---- install ----
            Expand-Archive -Path $zip -DestinationPath $tmp -Force
            $newExe = Join-Path $tmp "$bin.exe"
            if (-not (Test-Path $newExe)) { throw "tasker: archive does not contain $bin.exe" }
            try {
                Copy-Item $newExe $exe -Force
            } catch {
                throw "tasker: could not write $exe. Is $bin running? Close it and try again."
            }
        } finally {
            Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
        }

        $installed = $bin
        try { $installed = & $exe --version } catch { }
        Say "installed $installed to $exe"

        # The desktop app goes in the Start Menu.
        if ($bin -eq 'tasker-gui') {
            try {
                $link = (New-Object -ComObject WScript.Shell).CreateShortcut($shortcut)
                $link.TargetPath = $exe
                $link.WorkingDirectory = $installDir
                $link.Description = 'tasker: todo tracker'
                $link.Save()
                Say "added tasker to the Start Menu"
            } catch {
                Say "warning: could not add a Start Menu shortcut ($($_.Exception.Message)); run $exe"
            }
        }
    }

    # ---- PATH ----
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = if ($userPath) { $userPath -split ';' } else { @() }
    if (-not ($entries | Where-Object { $_.TrimEnd('\') -eq $installDir.TrimEnd('\') })) {
        $newPath = (@($entries | Where-Object { $_ }) + $installDir) -join ';'
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Say "added $installDir to your user PATH (new terminals pick it up)"
    }
    if (-not (($env:Path -split ';') -contains $installDir)) { $env:Path = "$env:Path;$installDir" }
    Say "run: $($bins -join ' or ')"
}

Install-Tasker
