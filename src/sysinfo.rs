use std::os::windows::process::CommandExt;
use std::process::Command;

use serde_json::Value;

use crate::i18n::Texts;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const COLLECT_SCRIPT: &str = r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = 'SilentlyContinue'
$result = [ordered]@{ elevated = $false; manufacturer = $null; model = $null; biosDate = $null; baseBoardDate = $null; hasBattery = $false; disks = @(); batteries = @(); diskError = $null; batteryError = $null }

try {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    $result.elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
} catch { }

try {
    $system = Get-CimInstance Win32_ComputerSystem -ErrorAction Stop
    $result.manufacturer = [string]$system.Manufacturer
    $result.model = [string]$system.Model
} catch { }

try {
    $bios = Get-CimInstance Win32_BIOS -ErrorAction Stop
    if ($null -ne $bios.ReleaseDate) {
        $date = [datetime]$bios.ReleaseDate
        if ($date.Year -gt 1980) { $result.biosDate = $date.ToString('yyyy-MM-dd') }
    }
} catch { }

try {
    $biosKey = Get-ItemProperty 'HKLM:\HARDWARE\DESCRIPTION\System\BIOS' -ErrorAction Stop
    $rawDate = [string]$biosKey.BaseBoardReleaseDate
    if (-not [string]::IsNullOrWhiteSpace($rawDate)) {
        try { $result.baseBoardDate = ([datetime]$rawDate).ToString('yyyy-MM-dd') }
        catch { $result.baseBoardDate = $rawDate }
    }
} catch { }

# Fallback: raw ATA SMART attribute 0x09 (Power-On Hours), available only when elevated.
$smartHours = @{}
try {
    foreach ($className in @('MSStorageDriver_ATAPISmartData', 'MSStorageDriver_FailurePredictData')) {
        foreach ($item in @(Get-CimInstance -Namespace root\wmi -ClassName $className -ErrorAction Stop)) {
            $bytes = @($item.VendorSpecific)
            if ($bytes.Count -lt 14) { continue }
            $name = [string]$item.InstanceName
            for ($i = 2; $i -le $bytes.Count - 12; $i += 12) {
                if ([int]$bytes[$i] -eq 9) {
                    $value = [int]$bytes[$i + 5] + ([int]$bytes[$i + 6] * 256)
                    if ($value -gt 0 -and -not $smartHours.ContainsKey($name)) {
                        $smartHours[$name] = [uint64]$value
                    }
                    break
                }
            }
        }
    }
} catch { }

try {
    foreach ($disk in @(Get-PhysicalDisk -ErrorAction Stop)) {
        $hours = $null
        try {
            $counter = $disk | Get-StorageReliabilityCounter -ErrorAction Stop
            if ($null -ne $counter.PowerOnHours) { $hours = [uint64]$counter.PowerOnHours }
        } catch { }

        # Second path: Get-Disk can expose counters when Get-PhysicalDisk does not.
        if ($null -eq $hours) {
            try {
                $serial = [string]$disk.SerialNumber
                $diskObj = Get-Disk -ErrorAction Stop | Where-Object {
                    $_.SerialNumber -and $serial -and $_.SerialNumber.Trim() -eq $serial.Trim()
                } | Select-Object -First 1
                if ($null -ne $diskObj) {
                    $counter2 = $diskObj | Get-StorageReliabilityCounter -ErrorAction Stop
                    if ($null -ne $counter2.PowerOnHours) { $hours = [uint64]$counter2.PowerOnHours }
                }
            } catch { }
        }

        # Third path: match the raw SMART entry by serial or model name.
        if ($null -eq $hours -and $smartHours.Count -gt 0) {
            $serial = ([string]$disk.SerialNumber).Trim()
            $model = [string]$disk.FriendlyName
            foreach ($key in $smartHours.Keys) {
                if (($serial -and $key.Contains($serial)) -or ($model -and $key.Contains($model))) {
                    $hours = $smartHours[$key]
                    break
                }
            }
        }

        $result.disks += [pscustomobject]@{
            name  = [string]$disk.FriendlyName
            media = [string]$disk.MediaType
            bytes = [uint64]$disk.Size
            hours = $hours
        }
    }
} catch {
    $result.diskError = $_.Exception.Message
}

try {
    $charges = @{}
    $batteryDevices = @(Get-CimInstance Win32_Battery -ErrorAction Stop)
    $result.hasBattery = $batteryDevices.Count -gt 0
    foreach ($battery in $batteryDevices) {
        $charges[[string]$battery.Name] = [uint32]$battery.EstimatedChargeRemaining
    }

    if ($result.hasBattery) {
        $tempFile = Join-Path $env:TEMP ("rusttime_batt_" + [guid]::NewGuid().ToString('N') + '.xml')
        & powercfg /batteryreport /output $tempFile /xml | Out-Null
        $xml = [xml](Get-Content -Path $tempFile -Raw -ErrorAction Stop)

        foreach ($battery in @($xml.BatteryReport.Batteries.Battery)) {
            $name = [string]$battery.Id
            $result.batteries += [pscustomobject]@{
                name   = $name
                cycles = if ($battery.CycleCount) { [uint32]$battery.CycleCount } else { $null }
                design = if ($battery.DesignCapacity) { [uint64]$battery.DesignCapacity } else { $null }
                full   = if ($battery.FullChargeCapacity) { [uint64]$battery.FullChargeCapacity } else { $null }
                charge = $charges[$name]
            }
        }

        if ($result.batteries.Count -eq 0) {
            foreach ($battery in $batteryDevices) {
                $result.batteries += [pscustomobject]@{
                    name   = [string]$battery.Name
                    cycles = $null
                    design = $null
                    full   = $null
                    charge = [uint32]$battery.EstimatedChargeRemaining
                }
            }
        }
        Remove-Item $tempFile -ErrorAction SilentlyContinue
    }
} catch {
    if ($result.hasBattery) { $result.batteryError = $_.Exception.Message }
}

$result | ConvertTo-Json -Depth 5 -Compress
"#;

#[derive(Clone, Default)]
pub struct DiskInfo {
    pub name: String,
    pub media: String,
    pub size_gb: f64,
    pub power_on_hours: Option<u64>,
}

#[derive(Clone, Default)]
pub struct BatteryInfo {
    pub name: String,
    pub cycles: Option<u32>,
    pub charge_percent: Option<u32>,
    pub design_capacity: Option<u64>,
    pub full_capacity: Option<u64>,
}

#[derive(Clone, Default)]
pub struct SystemInfo {
    pub disks: Vec<DiskInfo>,
    pub batteries: Vec<BatteryInfo>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub bios_date: Option<String>,
    pub baseboard_date: Option<String>,
    pub has_battery: bool,
    pub elevated: bool,
    pub disk_error: Option<String>,
    pub battery_error: Option<String>,
    pub loaded: bool,
}

pub fn collect(t: Texts) -> SystemInfo {
    let mut info = SystemInfo::default();

    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            COLLECT_SCRIPT,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(_) => {
            info.disk_error = Some(t.powershell_failed.to_owned());
            info.battery_error = Some(t.powershell_failed.to_owned());
            info.loaded = true;
            return info;
        }
    };

    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let json: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            info.disk_error = Some(if detail.is_empty() {
                t.parse_failed.to_owned()
            } else {
                detail
            });
            info.loaded = true;
            return info;
        }
    };

    info.elevated = json.get("elevated").and_then(Value::as_bool).unwrap_or(false);
    info.has_battery = json.get("hasBattery").and_then(Value::as_bool).unwrap_or(true);
    info.manufacturer = string_field(json.get("manufacturer"));
    info.model = string_field(json.get("model"));
    info.bios_date = string_field(json.get("biosDate"));
    info.baseboard_date = string_field(json.get("baseBoardDate"));
    info.disk_error = string_field(json.get("diskError"));
    info.battery_error = string_field(json.get("batteryError"));

    for value in values(json.get("disks")) {
        info.disks.push(DiskInfo {
            name: string_field(value.get("name")).unwrap_or_default(),
            media: string_field(value.get("media")).unwrap_or_else(|| t.disk_default.to_owned()),
            size_gb: value
                .get("bytes")
                .and_then(Value::as_u64)
                .map(|bytes| bytes as f64 / 1024.0 / 1024.0 / 1024.0)
                .unwrap_or(0.0),
            power_on_hours: value.get("hours").and_then(Value::as_u64),
        });
    }

    for value in values(json.get("batteries")) {
        info.batteries.push(BatteryInfo {
            name: string_field(value.get("name")).unwrap_or_else(|| t.battery_default.to_owned()),
            cycles: value.get("cycles").and_then(Value::as_u64).map(|n| n as u32),
            charge_percent: value
                .get("charge")
                .and_then(Value::as_u64)
                .map(|n| n as u32),
            design_capacity: value.get("design").and_then(Value::as_u64),
            full_capacity: value.get("full").and_then(Value::as_u64),
        });
    }

    info.loaded = true;
    info
}

fn values(field: Option<&Value>) -> Vec<&Value> {
    match field {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(object @ Value::Object(_)) => vec![object],
        _ => Vec::new(),
    }
}

fn string_field(field: Option<&Value>) -> Option<String> {
    field.and_then(Value::as_str).map(str::to_owned)
}
