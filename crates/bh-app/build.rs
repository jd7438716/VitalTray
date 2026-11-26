//! 把图标 / 版本信息 / 应用清单编进 exe（Windows 资源）。
//!
//! 路线：生成 `.rc` → Windows SDK 的 `rc.exe` 编成 `.res` → 交给链接器。
//! 刻意不引第三方构建依赖；找不到 `rc.exe`（比如没装 Windows SDK）时只警告不失败，
//! 这样源码构建仍然可用，只是 exe 不带资源和版本信息（`tools/check-exe.ps1` 会拦发布）。

use std::path::{Path, PathBuf};
use std::process::Command;

/// exe 内的图标资源 ID（窗口类、托盘兜底图标都按它加载）。
const ICON_ID: &str = "1";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let ico = manifest_dir.join("../../assets/vitaltray.ico");
    println!("cargo:rerun-if-changed={}", ico.display());

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }
    if !ico.is_file() {
        println!("cargo:warning=assets/vitaltray.ico 不存在，exe 将不带图标");
        return;
    }
    let Some(rc) = find_rc() else {
        println!("cargo:warning=找不到 rc.exe（Windows SDK），exe 将不带图标/版本信息/清单");
        return;
    };

    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let (a, b, c, d) = version_tuple(&version);
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default());

    let manifest = out.join("vitaltray.manifest");
    if std::fs::write(&manifest, manifest_xml(&version)).is_err() {
        println!("cargo:warning=写应用清单失败，exe 将不带清单（菜单会用经典样式）");
        return;
    }

    let rc_file = out.join("vitaltray.rc");
    if std::fs::write(&rc_file, rc_text(&ico, &manifest, (a, b, c, d), &version)).is_err() {
        println!("cargo:warning=写 .rc 失败，exe 将不带资源");
        return;
    }

    let res = out.join("vitaltray.res");
    let ok = Command::new(&rc)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res)
        .arg(&rc_file)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        println!("cargo:warning=rc.exe 编译资源失败，exe 将不带图标/版本信息/清单");
        return;
    }
    println!("cargo:rustc-link-arg={}", res.display());
}

/// `1.1.0` → `(1, 1, 0, 0)`（VERSIONINFO 要 4 段）。
fn version_tuple(v: &str) -> (u16, u16, u16, u16) {
    let mut parts = v.split('.').filter_map(|p| p.parse::<u16>().ok());
    (
        parts.next().unwrap_or(1),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        0,
    )
}

/// 找 Windows SDK 里的 rc.exe：先看 `RC` 环境变量，再按 SDK 版本从高到低找。
fn find_rc() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RC") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => "x64",
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };
    let mut roots: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(pf) = std::env::var(var) {
            roots.push(PathBuf::from(pf).join("Windows Kits").join("10").join("bin"));
        }
    }
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else { continue };
        let mut versions: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        versions.sort();
        for v in versions.iter().rev() {
            let candidate = v.join(arch).join("rc.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// .rc 里的路径要转义反斜杠，否则 `\b`、`\t` 会被当转义字符。
fn esc(p: &Path) -> String {
    p.display().to_string().replace('\\', "\\\\")
}

/// 文件说明：资源管理器/属性页按系统语言挑一份（与界面语言无关，改系统语言后自变）。
///
/// 语言 ID 用 LANGID 十六进制，代码页 1200（Unicode）；`rc.exe` 允许一个 VERSIONINFO
/// 里挂多个 StringFileInfo 块，正是给这种情况用的。
const DESCRIPTIONS: [(&str, &str); 10] = [
    ("0409", "VitalTray — wireless device battery and system temperature tray panel"),
    ("0804", "VitalTray — 无线外设电量与整机温度托盘面板"),
    ("0404", "VitalTray — 無線周邊電量與整機溫度系統匣面板"),
    ("0411", "VitalTray — ワイヤレス機器の残量と本体温度のトレイパネル"),
    ("0412", "VitalTray — 무선 장치 배터리와 본체 온도 트레이 패널"),
    ("0407", "VitalTray — Akkustand von Funkgeräten und Systemtemperaturen im Tray"),
    ("040C", "VitalTray — batterie des appareils sans fil et températures système dans la zone de notification"),
    ("0C0A", "VitalTray — batería de dispositivos inalámbricos y temperaturas del sistema en la bandeja"),
    ("0419", "VitalTray — заряд беспроводных устройств и температуры системы в трее"),
    ("0416", "VitalTray — bateria de dispositivos sem fio e temperaturas do sistema na bandeja"),
];

fn rc_text(
    ico: &Path,
    manifest: &Path,
    (a, b, c, d): (u16, u16, u16, u16),
    version: &str,
) -> String {
    format!(
        "#pragma code_page(65001)\n\
         {ICON_ID} ICON \"{ico}\"\n\
         {ICON_ID} 24 \"{manifest}\"\n\
         {ICON_ID} VERSIONINFO\n\
         FILEVERSION {a},{b},{c},{d}\n\
         PRODUCTVERSION {a},{b},{c},{d}\n\
         FILEOS 0x40004L\n\
         FILETYPE 0x1L\n\
         BEGIN\n\
         \x20 BLOCK \"StringFileInfo\"\n\
         \x20 BEGIN\n\
         {blocks}\
         \x20 END\n\
         \x20 BLOCK \"VarFileInfo\"\n\
         \x20 BEGIN\n\
         \x20   VALUE \"Translation\", {translations}\n\
         \x20 END\n\
         END\n",
        ico = esc(ico),
        manifest = esc(manifest),
        blocks = string_file_info_blocks(version),
        translations = translation_pairs(),
    )
}

fn string_file_info_blocks(version: &str) -> String {
    let mut out = String::new();
    for (lang, description) in DESCRIPTIONS {
        out.push_str(&format!(
            "    BLOCK \"{lang}04B0\"\n\
             \x20   BEGIN\n\
             \x20     VALUE \"CompanyName\", \"VitalTray\"\n\
             \x20     VALUE \"FileDescription\", \"{description}\"\n\
             \x20     VALUE \"FileVersion\", \"{version}\"\n\
             \x20     VALUE \"InternalName\", \"vitaltray\"\n\
             \x20     VALUE \"LegalCopyright\", \"MIT License\"\n\
             \x20     VALUE \"OriginalFilename\", \"vitaltray.exe\"\n\
             \x20     VALUE \"ProductName\", \"VitalTray\"\n\
             \x20     VALUE \"ProductVersion\", \"{version}\"\n\
             \x20   END\n"
        ));
    }
    out
}

fn translation_pairs() -> String {
    DESCRIPTIONS
        .iter()
        .map(|(lang, _)| format!("0x{lang}, 1200"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// 应用清单：视觉样式（comctl32 v6）+ 按显示器 DPI + Win10/11 支持声明。
fn manifest_xml(version: &str) -> String {
    let (a, b, c, _) = version_tuple(version);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="VitalTray.app" version="{a}.{b}.{c}.0" processorArchitecture="*"/>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{{1f676c76-80e1-4239-95bb-83d0f6d0da78}}"/>
      <supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>
"#
    )
}
