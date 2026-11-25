# Spike S4: tao file Excel co cac dinh dang de lech bang chinh Microsoft Excel (COM),
# va ghi lai chu Excel hien thi (Range.Text) cho tung o lam chuan thu ba.
#
# Cach dung: powershell -File make-tricky.ps1 <tricky.xlsx> <excel-text.json>
# File nay chi dung ky tu ASCII de PowerShell 5.1 doc dung ma khong can BOM.

param(
    [Parameter(Mandatory = $true)][string]$XlsxPath,
    [Parameter(Mandatory = $true)][string]$TextJsonPath
)

$ErrorActionPreference = 'Stop'
$XlsxPath = [IO.Path]::GetFullPath($XlsxPath)
$TextJsonPath = [IO.Path]::GetFullPath($TextJsonPath)
if (Test-Path $XlsxPath) { Remove-Item $XlsxPath -Force }

$headerRow = 2
$firstCol = 2
# Ten cot, dinh dang so (ma kieu en-US), gia tri cho 3 dong du lieu (dong 3, 4, 6; dong 5 de trong).
$d1 = ([datetime]'2026-03-05').ToOADate()
$d2 = ([datetime]'1999-12-31').ToOADate()
$d3 = ([datetime]'2026-11-18').ToOADate()
$columns = @(
    @{ name = 'so_text';        fmt = '@';                  v = @('0001', '0012', '0123') },
    @{ name = 'so_0000';        fmt = '0000';               v = @(1, 12, 123) },
    @{ name = 'tien_nhom';      fmt = '#,##0';              v = @(1234567, 999, 15000000) },
    @{ name = 'tien_thap_phan'; fmt = '#,##0.00';           v = @(1234567.5, 0.5, 1000) },
    @{ name = 'phan_tram';      fmt = '0.00%';              v = @(0.125, 1, 0.0333) },
    @{ name = 'ngay_ddmmyyyy';  fmt = 'dd/mm/yyyy';         v = @($d1, $d2, $d3) },
    @{ name = 'ngay_mac_dinh';  fmt = 'm/d/yyyy';           v = @($d1, $d2, $d3) },
    @{ name = 'ngay_gio';       fmt = 'dd/mm/yyyy hh:mm';   v = @(($d1 + 0.604166666666667), ($d2 + 0.999), $d3) },
    @{ name = 'so_general';     fmt = 'General';            v = @(15000000, 3.5, -7) },
    @{ name = 'so_thuc';        fmt = 'General';            v = @(0.1, 1234.5678, 1e-7) },
    @{ name = 'cong_thuc';      fmt = 'General';            v = @('=J3*2', '=J4*2', '=J6*2') },
    @{ name = 'khoang_trang';   fmt = '@';                  v = @('  abc  ', 'x y', ' z') },
    @{ name = 'logic';          fmt = 'General';            v = @($true, $false, $true) },
    @{ name = 'gio';            fmt = 'hh:mm';              v = @(0.604166666666667, 0.25, 0.999) },
    @{ name = 'tien_don_vi';    fmt = '#,##0 "VND"';        v = @(2500000, 0, 1) },
    @{ name = 'am_ngoac';       fmt = '#,##0;(#,##0)';      v = @(-1500, 1500, 0) },
    @{ name = 'ngay_text';      fmt = '@';                  v = @('05/03/2026', '31/12/1999', '18/11/2026') },
    @{ name = 'so_lon';         fmt = 'General';            v = @(123456789012, 12345678901234567, 0.000001234) },
    @{ name = '';               fmt = 'General';            v = @('cot khong tieu de', 'x', 'y') },
    @{ name = 'o_gop';          fmt = 'General';            v = @('gop', $null, 'le') }
)
$dataRows = @(3, 4, 6)

# Ghi lai cac tien trinh Excel co san de chi dong dung tien trinh do script nay mo.
$before = @(Get-Process EXCEL -ErrorAction SilentlyContinue | ForEach-Object { $_.Id })
$excel = New-Object -ComObject Excel.Application
$ownPids = @(Get-Process EXCEL -ErrorAction SilentlyContinue | Where-Object { $before -notcontains $_.Id } | ForEach-Object { $_.Id })
$excel.Visible = $false
$excel.DisplayAlerts = $false
try {
    $wb = $excel.Workbooks.Add()
    $ws = $wb.Worksheets.Item(1)
    $ws.Name = 'DuLieu'

    for ($i = 0; $i -lt $columns.Count; $i++) {
        $col = $firstCol + $i
        $c = $columns[$i]
        if ($c.name -ne '') {
            $h = $ws.Cells.Item($headerRow, $col)
            [void]$h.GetType().InvokeMember('Value2', [Reflection.BindingFlags]::SetProperty, $null, $h, @($c.name))
        }
        for ($r = 0; $r -lt $dataRows.Count; $r++) {
            $cell = $ws.Cells.Item($dataRows[$r], $col)
            $cell.NumberFormat = $c.fmt
            $value = $c.v[$r]
            if ($null -eq $value) { continue }
            # Gan qua InvokeMember: bo lien ket COM cua PowerShell nho kieu cua lan gan dau
            # (chuoi) va bao loi ep kieu khi lan sau la so.
            $member = if ($value -is [string] -and $value.StartsWith('=')) { 'Formula' } else { 'Value2' }
            [void]$cell.GetType().InvokeMember($member, [Reflection.BindingFlags]::SetProperty, $null, $cell, @($value))
        }
    }

    # O gop: cot cuoi, dong 3 va 4.
    $mergeCol = $firstCol + $columns.Count - 1
    $ws.Range($ws.Cells.Item(3, $mergeCol), $ws.Cells.Item(4, $mergeCol)).Merge()

    # Sheet thu hai khong duoc doc.
    $other = $wb.Worksheets.Add([Type]::Missing, $ws)
    $other.Name = 'Khac'
    $other.Cells.Item(1, 1).Value2 = 'khong doc sheet nay'
    $ws.Activate()

    $ws.Columns.AutoFit() | Out-Null
    $excel.Calculate()

    $text = [ordered]@{}
    foreach ($r in @($headerRow) + $dataRows) {
        for ($i = 0; $i -lt $columns.Count; $i++) {
            $col = $firstCol + $i
            $text["$r,$col"] = [string]$ws.Cells.Item($r, $col).Text
        }
    }

    $wb.SaveAs($XlsxPath, 51)
    $wb.Close($false)

    $payload = [ordered]@{
        excelVersion = [string]$excel.Version
        culture      = (Get-Culture).Name
        cells        = $text
    }
    [IO.File]::WriteAllText($TextJsonPath, ($payload | ConvertTo-Json -Depth 4), (New-Object Text.UTF8Encoding $false))
    Write-Output "Da tao $XlsxPath va $TextJsonPath"
}
finally {
    $excel.Quit()
    $cell = $null; $h = $null; $ws = $null; $other = $null; $wb = $null
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($excel)
    $excel = $null
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
    [GC]::Collect()
    Start-Sleep -Milliseconds 1500
    foreach ($id in $ownPids) {
        if (Get-Process -Id $id -ErrorAction SilentlyContinue) {
            Stop-Process -Id $id -Force
            Write-Output "Excel (PID $id) khong tu thoat, da buoc dong."
        }
    }
}
