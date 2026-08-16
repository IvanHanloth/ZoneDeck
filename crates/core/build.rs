//! 构建脚本：为核心 exe 嵌入 Windows 资源（应用清单、版本信息、进程图标）。
//!
//! 版本号只认 Cargo.toml：文件版本信息由 tauri-winres 取自 `CARGO_PKG_VERSION`，
//! 清单里的 `assemblyIdentity version="{VERSION}"` 由本脚本按同一来源填入。
//!
//! manifest.xml 只能用 ASCII，且 `assemblyIdentity` 必须是 `assembly` 的第一个子元素——
//! 中文注释在嵌入时会被按非 UTF-8 编码写坏，两者都会让 exe 以「并行配置不正确」拒绝启动。

/// 清单的 `assemblyIdentity` 只接受纯数字四段号：`3.1.0-rc.1` → `3.1.0.0`。
/// 与安装包的 `MyAppVersion4` 同一套规则（见 scripts/version.ps1）。
fn manifest_version(version: &str) -> String {
    let numeric = version.split('-').next().unwrap_or(version);
    format!("{numeric}.0")
}

fn main() {
    println!("cargo:rerun-if-changed=manifest.xml");
    println!("cargo:rerun-if-changed=icon.ico");
    // 声明了 rerun-if-changed 就得自己盯住版本号：否则改完 Cargo.toml 本脚本不会重跑，
    // 清单里会留着上一个版本号。
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo 未提供 CARGO_PKG_VERSION");
    let manifest = std::fs::read_to_string("manifest.xml")
        .expect("读取 manifest.xml 失败")
        .replace("{VERSION}", &manifest_version(&version));

    let mut res = tauri_winres::WindowsResource::new();
    res.set_icon("icon.ico");
    res.set_manifest(&manifest);
    res.set("ProductName", "ZoneDeck");
    res.set("FileDescription", "ZoneDeck 核心服务");
    res.set("CompanyName", "Ivan Hanloth");
    res.set(
        "LegalCopyright",
        "Copyright © 2022-2026 Ivan Hanloth All Rights Reserved.",
    );
    res.set("OriginalFilename", "ZoneDeck.exe");
    res.compile().expect("嵌入 Windows 资源失败");
}
