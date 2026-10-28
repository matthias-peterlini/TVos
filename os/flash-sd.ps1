# Writes the TVos Raspberry Pi image to a microSD card. Run from an administrator PowerShell:
#   powershell -ExecutionPolicy Bypass -File os\flash-sd.ps1 -DiskNumber 1
# Every byte on the chosen card is erased. The script refuses system disks, non-removable disks and disks over 256 GB.
param(
    [Parameter(Mandatory = $true)][int]$DiskNumber,
    [string]$Image = "$PSScriptRoot\..\out\tvos-rpi3.img"
)
$ErrorActionPreference = 'Stop'

$img = Get-Item $Image
$disk = Get-Disk -Number $DiskNumber
if ($disk.IsBoot -or $disk.IsSystem) { throw "Disk $DiskNumber is the Windows disk: refusing." }
if ($disk.BusType -notin @('USB', 'SD', 'MMC')) { throw "Disk $DiskNumber is on bus $($disk.BusType), not a card reader: refusing." }
if ($disk.Size -eq 0) { throw "No card in reader $DiskNumber." }
if ($disk.Size -gt 256GB) { throw "Disk $DiskNumber is larger than 256 GB: refusing." }
if ($img.Length -gt $disk.Size) { throw "The image does not fit on the card." }
"Card: disk $DiskNumber, $($disk.FriendlyName), {0:N1} GB" -f ($disk.Size / 1GB)
"Image: $($img.FullName), {0:N0} MB" -f ($img.Length / 1MB)

# Removing the partitions makes Windows release the card's drive letters, so the raw device can be written
if ($disk.PartitionStyle -ne 'RAW') { Clear-Disk -Number $DiskNumber -RemoveData -RemoveOEM -Confirm:$false }

Add-Type -Namespace TVos -Name Raw -MemberDefinition @'
[DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern Microsoft.Win32.SafeHandles.SafeFileHandle CreateFile(string name, uint access, uint share, IntPtr sec, uint disposition, uint flags, IntPtr template);
'@
$h = [TVos.Raw]::CreateFile("\\.\PhysicalDrive$DiskNumber", 0xC0000000, 3, [IntPtr]::Zero, 3, 0, [IntPtr]::Zero)
if ($h.IsInvalid) { throw "Cannot open the card (run PowerShell as administrator)." }
$out = New-Object IO.FileStream($h, [IO.FileAccess]::ReadWrite)
$in = [IO.File]::OpenRead($img.FullName)
$buf = New-Object byte[] (4MB)
$done = 0L
try {
    while (($n = $in.Read($buf, 0, $buf.Length)) -gt 0) {
        # Raw devices take whole 512-byte sectors only
        $len = [int]([math]::Ceiling($n / 512) * 512)
        if ($len -gt $n) { [Array]::Clear($buf, $n, $len - $n) }
        $out.Write($buf, 0, $len)
        $done += $n
        Write-Progress -Activity "Writing TVos" -PercentComplete ([int]($done * 100 / $img.Length))
    }
    $out.Flush()
} finally {
    $in.Close(); $out.Close()
}
Update-Disk -Number $DiskNumber
"Done: $([math]::Round($done / 1MB)) MB written. Remove the card and put it in the Raspberry Pi."
