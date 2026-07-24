param(
  [ValidateRange(0.75, 0.95)]
  [double]$TargetOccupancy = 0.90,
  [ValidateRange(256, 2048)]
  [int]$OutputSize = 1024
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$projectRoot = Split-Path -Parent $PSScriptRoot
$iconPath = Join-Path $projectRoot 'design\brand\long-jieya-icon.png'
$tempPath = Join-Path $projectRoot 'design\brand\long-jieya-icon.normalized.png'
$source = [Drawing.Bitmap]::new([string]$iconPath)

try {
  $minX = $source.Width
  $minY = $source.Height
  $maxX = -1
  $maxY = -1

  for ($y = 0; $y -lt $source.Height; $y++) {
    for ($x = 0; $x -lt $source.Width; $x++) {
      if ($source.GetPixel($x, $y).A -gt 8) {
        if ($x -lt $minX) { $minX = $x }
        if ($x -gt $maxX) { $maxX = $x }
        if ($y -lt $minY) { $minY = $y }
        if ($y -gt $maxY) { $maxY = $y }
      }
    }
  }

  if ($maxX -lt $minX -or $maxY -lt $minY) {
    throw 'The brand icon contains no visible pixels.'
  }

  $contentWidth = $maxX - $minX + 1
  $contentHeight = $maxY - $minY + 1
  $cropSide = [Math]::Ceiling([Math]::Max($contentWidth, $contentHeight) / $TargetOccupancy)
  $cropSide = [Math]::Min($cropSide, [Math]::Min($source.Width, $source.Height))
  $centerX = ($minX + $maxX) / 2
  $centerY = ($minY + $maxY) / 2
  $cropX = [Math]::Max(0, [Math]::Min($source.Width - $cropSide, [Math]::Round($centerX - $cropSide / 2)))
  $cropY = [Math]::Max(0, [Math]::Min($source.Height - $cropSide, [Math]::Round($centerY - $cropSide / 2)))

  $output = [Drawing.Bitmap]::new($OutputSize, $OutputSize, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
  try {
    $graphics = [Drawing.Graphics]::FromImage($output)
    try {
      $graphics.Clear([Drawing.Color]::Transparent)
      $graphics.CompositingMode = [Drawing.Drawing2D.CompositingMode]::SourceCopy
      $graphics.CompositingQuality = [Drawing.Drawing2D.CompositingQuality]::HighQuality
      $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
      $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
      $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::HighQuality
      $destination = [Drawing.Rectangle]::new(0, 0, $OutputSize, $OutputSize)
      $sourceRect = [Drawing.Rectangle]::new($cropX, $cropY, $cropSide, $cropSide)
      $graphics.DrawImage($source, $destination, $sourceRect, [Drawing.GraphicsUnit]::Pixel)
    } finally {
      $graphics.Dispose()
    }
    $output.Save($tempPath, [Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $output.Dispose()
  }
} finally {
  $source.Dispose()
}

Move-Item -LiteralPath $tempPath -Destination $iconPath -Force
Write-Output "Normalized brand icon to $OutputSize x $OutputSize with $([Math]::Round($TargetOccupancy * 100))% visual occupancy."
