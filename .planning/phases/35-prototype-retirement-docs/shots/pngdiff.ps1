# Pixel diff of PNGs.
#   pngdiff.ps1 -A a.png -B b.png            one pair
#   pngdiff.ps1 -Dir before -Dir2 after      every same-named PNG pair in two folders
# Prints "<name>: <n> differing pixels, bbox x0,y0-x1,y1" or "<name>: identical".
# Exit 0 when everything is identical, 1 on any difference, 2 on a size mismatch.
param([string]$A, [string]$B, [string]$Dir, [string]$Dir2)
Add-Type -AssemblyName System.Drawing
$script:code = 0

function Diff-Pair([string]$pa, [string]$pb) {
    $name = [IO.Path]::GetFileName($pa)
    $ba = New-Object System.Drawing.Bitmap ((Resolve-Path $pa).Path)
    $bb = New-Object System.Drawing.Bitmap ((Resolve-Path $pb).Path)
    try {
        if ($ba.Width -ne $bb.Width -or $ba.Height -ne $bb.Height) {
            Write-Output "${name}: size differs $($ba.Width)x$($ba.Height) vs $($bb.Width)x$($bb.Height)"
            $script:code = [Math]::Max($script:code, 2)
            return
        }
        $w = $ba.Width; $h = $ba.Height
        $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
        $fmt = [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
        $da = $ba.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, $fmt)
        $db = $bb.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, $fmt)
        $stride = $da.Stride
        $n = $stride * $h
        $xa = New-Object byte[] $n
        $xb = New-Object byte[] $n
        [Runtime.InteropServices.Marshal]::Copy($da.Scan0, $xa, 0, $n)
        [Runtime.InteropServices.Marshal]::Copy($db.Scan0, $xb, 0, $n)
        $ba.UnlockBits($da); $bb.UnlockBits($db)
        $cnt = 0; $x0 = $w; $y0 = $h; $x1 = -1; $y1 = -1
        for ($y = 0; $y -lt $h; $y++) {
            $row = $y * $stride
            for ($x = 0; $x -lt $w; $x++) {
                $i = $row + 4 * $x
                if ($xa[$i] -ne $xb[$i] -or $xa[$i+1] -ne $xb[$i+1] -or $xa[$i+2] -ne $xb[$i+2] -or $xa[$i+3] -ne $xb[$i+3]) {
                    $cnt++
                    if ($x -lt $x0) { $x0 = $x }
                    if ($x -gt $x1) { $x1 = $x }
                    if ($y -lt $y0) { $y0 = $y }
                    if ($y -gt $y1) { $y1 = $y }
                }
            }
        }
        if ($cnt -eq 0) { Write-Output "${name}: identical"; return }
        Write-Output "${name}: $cnt differing pixels, bbox $x0,$y0-$x1,$y1"
        $script:code = [Math]::Max($script:code, 1)
    } finally { $ba.Dispose(); $bb.Dispose() }
}

if ($Dir) {
    foreach ($f in Get-ChildItem -Path $Dir -Filter *.png | Sort-Object Name) {
        $other = Join-Path $Dir2 $f.Name
        if (-not (Test-Path $other)) { Write-Output "$($f.Name): missing in $Dir2"; $script:code = [Math]::Max($script:code, 1); continue }
        Diff-Pair $f.FullName $other
    }
}
else { Diff-Pair $A $B }
exit $script:code
