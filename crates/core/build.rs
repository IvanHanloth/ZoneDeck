//! 构建脚本：为核心 exe（core.exe）嵌入 Windows 资源。
//!
//! - 应用清单（manifest.xml）：PerMonitorV2 DPI 感知、长路径、系统版本声明、asInvoker；
//! - 版本信息（取自 Cargo 包版本）与产品元数据——正规化二进制，降低杀软误报；
//! - 进程图标（复用仓库根目录 icon.ico）。

fn main() {
    println!("cargo:rerun-if-changed=manifest.xml");
    println!("cargo:rerun-if-changed=../../icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest = std::fs::read_to_string("manifest.xml").expect("读取 manifest.xml 失败");

    let mut res = tauri_winres::WindowsResource::new();
    res.set_icon("../../icon.ico");
    res.set_manifest(&manifest);
    res.set("ProductName", "Boss Key");
    res.set("FileDescription", "Boss Key 核心服务");
    res.set("CompanyName", "Ivan Hanloth");
    res.set("LegalCopyright", "Copyright © 2022-2026 Ivan Hanloth All Rights Reserved.");
    res.set("OriginalFilename", "core.exe");
    res.compile().expect("嵌入 Windows 资源失败");
}
