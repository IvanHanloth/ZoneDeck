//! 数据文件（配置、日志、恢复文件、缓存）的目录定位。
//!
//! - **安装版**用 `%APPDATA%\ZoneDeck`。
//! - **便携版**用 exe 同目录；写不进去时退回 `%APPDATA%\ZoneDeck`。
//!
//! 靠程序目录里有没有 [`INSTALLED_MARKER`] 或卸载程序 `unins*.exe` 来分辨。
//! 判断依据是文件而非进程权限，核心与配置程序因此必然得出同一结果。

use std::path::{Path, PathBuf};

pub const CONFIG_FILE_NAME: &str = "config.json";
pub const USER_DIR_NAME: &str = "ZoneDeck";
/// 改名（Boss Key → ZoneDeck）前的用户目录名，仅用于迁移旧数据。
pub const LEGACY_USER_DIR_NAME: &str = "BossKey";
/// 安装版标记文件，由安装包放进程序目录，卸载时随之移除。
pub const INSTALLED_MARKER: &str = "installed.marker";

/// 数据目录为何是它。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDirKind {
    /// 安装版，数据在用户目录。
    Installed,
    /// 便携版，数据在程序目录。
    Portable,
    /// 便携版，但程序目录写不进去，退回用户目录。
    PortableFallback,
}

/// 数据目录的定位结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDir {
    /// 实际使用的目录。
    pub dir: PathBuf,
    pub kind: DataDirKind,
    /// 程序目录。[`DataDirKind::PortableFallback`] 时即写不进去的那个。
    pub program_dir: PathBuf,
}

/// 当前 exe 所在目录；取不到时退回当前工作目录。
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 用户目录 `%APPDATA%\ZoneDeck`；取不到 `%APPDATA%` 时退回 exe 同目录。
pub fn user_data_dir() -> PathBuf {
    match std::env::var_os("APPDATA") {
        Some(appdata) if !appdata.is_empty() => PathBuf::from(appdata).join(USER_DIR_NAME),
        _ => exe_dir(),
    }
}

/// 程序目录里有没有安装痕迹：[`INSTALLED_MARKER`] 或卸载程序 `unins*.exe`。
/// 后者序号随重复安装递增，故按前缀匹配。
pub fn is_installed(program_dir: &Path) -> bool {
    if program_dir.join(INSTALLED_MARKER).exists() {
        return true;
    }
    let Ok(entries) = std::fs::read_dir(program_dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        name.starts_with("unins") && name.ends_with(".exe")
    })
}

/// 当前进程能否在 `dir` 下建文件。探针文件用后即删。
pub fn dir_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".ZoneDeck-write-probe-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(file) => {
            drop(file);
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 把程序目录里的旧配置搬到用户目录：先复制，再尽力删掉原文件。
/// 目标已有配置就不动它。
fn migrate_config(program_dir: &Path, user_dir: &Path) {
    let old = program_dir.join(CONFIG_FILE_NAME);
    let new = user_dir.join(CONFIG_FILE_NAME);
    if !old.exists() || new.exists() {
        return;
    }
    if std::fs::copy(&old, &new).is_ok() {
        let _ = std::fs::remove_file(&old);
    }
}

/// 定位数据目录；`installed` 与 `portable_writable` 由调用方探测，便于测试注入。
pub fn resolve_data_dir(
    program_dir: &Path,
    user_dir: &Path,
    installed: bool,
    portable_writable: bool,
) -> DataDir {
    let located = |dir: PathBuf, kind: DataDirKind| DataDir {
        dir,
        kind,
        program_dir: program_dir.to_path_buf(),
    };
    if !installed && portable_writable {
        return located(program_dir.to_path_buf(), DataDirKind::Portable);
    }
    let kind = if installed {
        DataDirKind::Installed
    } else {
        DataDirKind::PortableFallback
    };
    if std::fs::create_dir_all(user_dir).is_err() {
        // 用户目录也建不出来时无处可去。
        return located(program_dir.to_path_buf(), kind);
    }
    migrate_config(program_dir, user_dir);
    located(user_dir.to_path_buf(), kind)
}

/// 把改名前的用户目录 `%APPDATA%\BossKey` 迁到 `%APPDATA%\ZoneDeck`。
/// 新目录已存在或旧目录不存在则不动。整体重命名失败时退回复制配置与恢复文件，
/// 旧目录连同日志留在原处。
pub fn migrate_legacy_user_dir(new_dir: &Path) {
    let Some(appdata) = new_dir.parent() else {
        return;
    };
    let legacy_dir = appdata.join(LEGACY_USER_DIR_NAME);
    if new_dir.exists() || !legacy_dir.exists() {
        return;
    }
    if std::fs::rename(&legacy_dir, new_dir).is_ok() {
        return;
    }
    if std::fs::create_dir_all(new_dir).is_err() {
        return;
    }
    for name in [CONFIG_FILE_NAME, "recovery.json"] {
        let old = legacy_dir.join(name);
        let new = new_dir.join(name);
        if old.exists() && !new.exists() {
            let _ = std::fs::copy(&old, &new);
        }
    }
}

/// 本次运行使用的数据目录，核心与配置程序共用。
pub fn locate() -> DataDir {
    let program_dir = exe_dir();
    let installed = is_installed(&program_dir);
    let user_dir = user_data_dir();
    // 用户目录本次不一定被选中，但旧数据只要还在就顺手迁走。
    migrate_legacy_user_dir(&user_dir);
    // 安装版结果一样是用户目录，不必再往程序目录里试写。
    let portable_writable = !installed && dir_writable(&program_dir);
    resolve_data_dir(&program_dir, &user_dir, installed, portable_writable)
}

pub fn data_dir() -> PathBuf {
    locate().dir
}

pub fn config_path() -> PathBuf {
    data_dir().join(CONFIG_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_data_dir_sits_under_appdata() {
        let dir = user_data_dir();
        if let Some(appdata) = std::env::var_os("APPDATA") {
            assert_eq!(dir, PathBuf::from(appdata).join(USER_DIR_NAME));
        }
    }
}
