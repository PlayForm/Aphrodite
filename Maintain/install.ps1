#!/usr/bin/env pwsh
# aphrodite - minimal one-command local install (PowerShell)
#
# Native equivalent of install.sh - runs the same on Windows PowerShell (5.1+)
# and cross-platform PowerShell (pwsh 7+ on macOS/Linux). This is the single
# source of truth for the Windows install flow; Maintain/install.bat is a
# thin wrapper that calls this script.
#
# Installs from a local clone or release build. Expects:
#   target/release/aphrodite(.exe)   (the Rust binary)
#   plugins/aphrodite/                (the Hermes plugin)
#   profiles/*/                       (7 profile directories)

$ErrorActionPreference = 'Stop'

$OnWindows = -not (Test-Path variable:IsWindows) -or $IsWindows
$BinaryName = if ($OnWindows) { 'aphrodite.exe' } else { 'aphrodite' }

$Repo = if ($env:REPO) { $env:REPO } else { (Resolve-Path (Join-Path $PSScriptRoot '..')).Path }
$Hermes = if ($env:HERMES) { $env:HERMES } else { Join-Path $HOME '.hermes' }
$BinaryDest = Join-Path $Hermes "aphrodite/$BinaryName"
$PluginSrc = Join-Path $Repo 'plugins/aphrodite'
$SkillsSrc = Join-Path $PluginSrc 'skills'

Write-Host '=== aphrodite install ==='
Write-Host "  repo:   $Repo"
Write-Host "  hermes: $Hermes"

# Link (or copy as a fallback) a directory - junction on Windows, symlink
# elsewhere. Neither requires admin rights for a directory junction.
function Set-DirLink {
	param([string]$Link, [string]$Target)

	if (Test-Path $Link) {
		Remove-Item -Force -Recurse $Link -ErrorAction SilentlyContinue
	}
	New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Link) | Out-Null

	try {
		if ($OnWindows) {
			New-Item -ItemType Junction -Path $Link -Target $Target -ErrorAction Stop | Out-Null
		} else {
			New-Item -ItemType SymbolicLink -Path $Link -Target $Target -ErrorAction Stop | Out-Null
		}
		return $true
	} catch {
		Write-Host "  WARN: link failed for $Link, copying instead ($_)"
		Copy-Item -Recurse -Force $Target $Link
		return $false
	}
}

# --- 1. Binary ----------------------------------------------------------
$BuiltBinary = Join-Path $Repo "target/release/$BinaryName"
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $BinaryDest) | Out-Null
if (Test-Path $BuiltBinary) {
	Copy-Item -Force $BuiltBinary $BinaryDest
	if (-not $OnWindows) { & chmod +x $BinaryDest }
	$size = [math]::Round((Get-Item $BinaryDest).Length / 1MB, 1)
	Write-Host "  binary: $BinaryDest (${size}MB)"
} else {
	Write-Host "  binary: SKIP - no release build at target/release/$BinaryName"
	Write-Host '          Run: cargo build --release -p aphrodite'
}

# --- 2. Plugin link -------------------------------------------------------
Set-DirLink -Link (Join-Path $Hermes 'plugins/aphrodite') -Target $PluginSrc | Out-Null
Write-Host "  plugin: $Hermes/plugins/aphrodite -> $PluginSrc"

# --- 3. Skills (hermes namespace) -----------------------------------------
Set-DirLink -Link (Join-Path $Hermes 'skills/hermes') -Target $SkillsSrc | Out-Null
Write-Host "  skills: $Hermes/skills/hermes -> $SkillsSrc"

# --- 4. Profiles -----------------------------------------------------------
# 7 pre-configured profiles ship inside the repo under profiles/. Rather than
# recreating them from scratch, link the whole directory so config.yaml + any
# state-driven cache/log stays inside the repo.
$ProfileNames = @(
	'barebone', 'proxy-cache', 'proxy-token',
	'compress-off', 'compress-light', 'compress-medium', 'compress-aggressive'
)

foreach ($name in $ProfileNames) {
	$profile = "aphrodite-$name"
	$src = Join-Path $Repo "profiles/$profile"
	$dst = Join-Path $Hermes "profiles/$profile"

	if (-not (Test-Path $src)) {
		Write-Host "  profile: $profile - SKIP (no directory at $src)"
		continue
	}

	Set-DirLink -Link $dst -Target $src | Out-Null

	# Ensure the plugin is listed in the profile's config
	try { & hermes plugins enable aphrodite --profile $profile 2>$null } catch {}

	Write-Host "  profile: $profile OK"
}

# Also enable in the default (active) profile
try { & hermes plugins enable aphrodite 2>$null } catch {}

Write-Host ''
Write-Host '=== done ==='
Write-Host '  Launch: hermes --profile aphrodite-compress-aggressive'
Write-Host '  Proxy:  hermes --profile aphrodite-proxy-token'
Write-Host '  Debug:  $env:APHRODITE_DEBUG=1; hermes --profile aphrodite-compress-aggressive'
