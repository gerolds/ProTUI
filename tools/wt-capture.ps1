# ADR: Opens one Windows Terminal window at a set size, runs a program in it, saves a PNG of the window via PrintWindow, then closes the window, also when the capture fails, so no program is left running.
# ADR: The program runs through a temporary .cmd wrapper, because wt.exe reads `;` and option-like words after the program path as its own arguments.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -Command "& tools/wt-capture.ps1 -Exe <path> [-ExeArgs 'a','b'] [-Profile <name>] [-Cols 120] [-Rows 40] [-Wait 3] -Out <png>"
# Use -Command, not -File: with -File, PowerShell passes `-ExeArgs a,b` as the one string "a,b".
# -Profile names a Windows Terminal profile; without it the window uses the default profile. Windows Terminal reads
# fragment profiles (JSON under %LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\<folder>\) only when it starts:
# after adding or changing one, close every Windows Terminal window before capturing.
param(
    [Parameter(Mandatory)] [string] $Exe,
    [string[]] $ExeArgs = @(),
    [string] $Profile = "",
    [int] $Cols = 120,
    [int] $Rows = 40,
    [Parameter(Mandatory)] [string] $Out,
    [double] $Wait = 3.0
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class W {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Ri, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[void][W]::SetProcessDPIAware()
# Agent shells export NO_COLOR=1 and TERM=dumb; Windows Terminal hands the caller's environment to the new tab, and crossterm then drops every colour.
Remove-Item Env:NO_COLOR, Env:TERM -ErrorAction SilentlyContinue
$title = "prototype-capture-" + [guid]::NewGuid().ToString("N").Substring(0, 8)
# The wrapper quotes each argument and doubles `%`, so cmd passes the arguments through unchanged; on a non-zero exit
# it shows the code for 10 s, so the capture shows the failure.
$quoted = @($ExeArgs | ForEach-Object { '"' + $_.Replace('%', '%%') + '"' })
$wrapper = Join-Path ([IO.Path]::GetTempPath()) "$title.cmd"
Set-Content -Path $wrapper -Encoding ASCII -Value @(
    "@echo off",
    ('"' + $Exe.Replace('%', '%%') + '" ' + ($quoted -join " ")),
    "if errorlevel 1 (echo exit %errorlevel% & timeout /t 10 >nul)"
)
# GDI+ refuses to save into a missing folder, and .NET resolves a relative path against the process folder, not the
# PowerShell location; resolve the path and create the folder before the window opens.
$Out = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Out)
[void](New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Out))
$h = [IntPtr]::Zero
try {
    $wtArgs = @("-w", "new", "--size", "$Cols,$Rows")
    if ($Profile) { $wtArgs += @("-p", "`"$Profile`"") }
    $wtArgs += @("--title", $title, "--suppressApplicationTitle", "`"$wrapper`"")
    Start-Process wt.exe -ArgumentList $wtArgs
    $deadline = (Get-Date).AddSeconds(20)
    # PowerShell turns $null into "" for string parameters; FindWindow needs a real null class name.
    while ($h -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 200; $h = [W]::FindWindow([NullString]::Value, $title) }
    if ($h -eq [IntPtr]::Zero) { throw "window '$title' not found" }
    Start-Sleep -Milliseconds ([int]($Wait * 1000))
    # A window found right after it opens can report an empty rectangle; look it up again until it has a size.
    $r = New-Object W+R
    $w = 0; $hh = 0
    for ($i = 0; $i -lt 25 -and ($w -le 0 -or $hh -le 0); $i++) {
        [void][W]::GetWindowRect($h, [ref]$r)
        $w = $r.Ri - $r.L; $hh = $r.B - $r.T
        if ($w -le 0 -or $hh -le 0) { Start-Sleep -Milliseconds 200; $h = [W]::FindWindow([NullString]::Value, $title) }
    }
    if ($w -le 0 -or $hh -le 0) { throw "window '$title' has no size: h=$h rect=$($r.L),$($r.T),$($r.Ri),$($r.B)" }
    $bmp = New-Object System.Drawing.Bitmap $w, $hh
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    # Flag 2 (PW_RENDERFULLCONTENT) captures the GPU-drawn content of Windows Terminal.
    $ok = [W]::PrintWindow($h, $dc, 2)
    $g.ReleaseHdc($dc); $g.Dispose()
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    "captured=$ok size=${w}x${hh} -> $Out"
}
finally {
    # WM_CLOSE ends the tab and the program in it.
    if ($h -ne [IntPtr]::Zero) { [void][W]::PostMessage($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) }
    Remove-Item $wrapper -ErrorAction SilentlyContinue
}
