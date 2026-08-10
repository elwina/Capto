# Generate Capto NSIS installer branding bitmaps from brand masters.
# Usage: .\scripts\gen-nsis-brand.ps1
#
# Outputs (NSIS MUI sizes):
#   apps/desktop/src-tauri/windows/nsis/sidebar.bmp  164x314
#   apps/desktop/src-tauri/windows/nsis/header.bmp   150x57

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$RepoRoot = Split-Path -Parent $PSScriptRoot
$BrandDir = Join-Path $RepoRoot "brand"
$OutDir = Join-Path $RepoRoot "apps\desktop\src-tauri\windows\nsis"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$MarkPath = Join-Path $BrandDir "capto-mark.png"
$LockupPath = Join-Path $BrandDir "capto-lockup.png"
if (-not (Test-Path -LiteralPath $MarkPath)) { throw "Missing $MarkPath" }

$Bg = [System.Drawing.Color]::FromArgb(255, 0, 0, 0)
$Brand = [System.Drawing.Color]::FromArgb(255, 0xA7, 0x8B, 0xFF)

function Save-Bmp24 {
    param(
        [System.Drawing.Bitmap]$Bitmap,
        [string]$Path
    )
    # Flatten to 24-bit RGB BMP (NSIS MUI prefers classic BMP without alpha).
    $flat = New-Object System.Drawing.Bitmap $Bitmap.Width, $Bitmap.Height, ([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $g = [System.Drawing.Graphics]::FromImage($flat)
    try {
        $g.Clear($Bg)
        $g.DrawImage($Bitmap, 0, 0, $Bitmap.Width, $Bitmap.Height)
    } finally {
        $g.Dispose()
    }
    $flat.Save($Path, [System.Drawing.Imaging.ImageFormat]::Bmp)
    $flat.Dispose()
}

function Draw-CenteredImage {
    param(
        [System.Drawing.Graphics]$Graphics,
        [System.Drawing.Image]$Image,
        [int]$MaxWidth,
        [int]$MaxHeight,
        [int]$CenterX,
        [int]$CenterY
    )
    $scale = [Math]::Min($MaxWidth / [double]$Image.Width, $MaxHeight / [double]$Image.Height)
    $w = [int][Math]::Round($Image.Width * $scale)
    $h = [int][Math]::Round($Image.Height * $scale)
    $x = $CenterX - [int]($w / 2)
    $y = $CenterY - [int]($h / 2)
    $Graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $Graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $Graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $Graphics.DrawImage($Image, $x, $y, $w, $h)
}

$mark = [System.Drawing.Image]::FromFile($MarkPath)
try {
    # --- sidebar 164x314 ---
    $sidebar = New-Object System.Drawing.Bitmap 164, 314, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $sg = [System.Drawing.Graphics]::FromImage($sidebar)
    try {
        $sg.Clear($Bg)
        # Mark in upper-middle area
        Draw-CenteredImage -Graphics $sg -Image $mark -MaxWidth 112 -MaxHeight 112 -CenterX 82 -CenterY 118

        $font = New-Object System.Drawing.Font "Segoe UI Semibold", 14, ([System.Drawing.FontStyle]::Bold)
        try {
            $brush = New-Object System.Drawing.SolidBrush $Brand
            try {
                $fmt = New-Object System.Drawing.StringFormat
                $fmt.Alignment = [System.Drawing.StringAlignment]::Center
                $fmt.LineAlignment = [System.Drawing.StringAlignment]::Near
                $sg.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
                $sg.DrawString("CAPTO", $font, $brush, (New-Object System.Drawing.RectangleF 0, 190, 164, 40), $fmt)
                $sub = New-Object System.Drawing.Font "Segoe UI", 8
                try {
                    $muted = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 0xA8, 0xA0, 0xBC))
                    try {
                        $sg.DrawString("Screen recorder", $sub, $muted, (New-Object System.Drawing.RectangleF 0, 218, 164, 28), $fmt)
                    } finally { $muted.Dispose() }
                } finally { $sub.Dispose() }
            } finally { $brush.Dispose(); $fmt.Dispose() }
        } finally { $font.Dispose() }
    } finally { $sg.Dispose() }

    $sidebarPath = Join-Path $OutDir "sidebar.bmp"
    Save-Bmp24 -Bitmap $sidebar -Path $sidebarPath
    $sidebar.Dispose()
    Write-Host "Wrote $sidebarPath (164x314)"

    # --- header 150x57 ---
    $header = New-Object System.Drawing.Bitmap 150, 57, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $hg = [System.Drawing.Graphics]::FromImage($header)
    try {
        $hg.Clear($Bg)
        Draw-CenteredImage -Graphics $hg -Image $mark -MaxWidth 36 -MaxHeight 36 -CenterX 24 -CenterY 28

        $font = New-Object System.Drawing.Font "Segoe UI Semibold", 12, ([System.Drawing.FontStyle]::Bold)
        try {
            $brush = New-Object System.Drawing.SolidBrush $Brand
            try {
                $hg.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
                $hg.DrawString("Capto", $font, $brush, 48, 16)
            } finally { $brush.Dispose() }
        } finally { $font.Dispose() }
    } finally { $hg.Dispose() }

    $headerPath = Join-Path $OutDir "header.bmp"
    Save-Bmp24 -Bitmap $header -Path $headerPath
    $header.Dispose()
    Write-Host "Wrote $headerPath (150x57)"
} finally {
    $mark.Dispose()
}

Write-Host "Done."
