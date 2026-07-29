//! 数据文件（配置、日志、恢复文件、缓存）的目录定位。
//!
//! 安装版与便携版分开对待：
//!
//! - **安装版**用 `%APPDATA%\BossKey`。安装包可以装进 `Program Files`，那里普通权限
//!   进程不可写，配置程序每次保存都会得到 `os error 5`。
//! - **便携版**用 exe 同目录，拷走整个文件夹就带走了全部设置。
//!   目录写不进去时退回 `%APPDATA%\BossKey`，程序照常能用，界面据此提示这是权限问题。
//!
//! 靠程序目录里有没有安装痕迹来分辨：安装包会放一份 [`INSTALLED_MARKER`]，
//! 卸载程序 `unins*.exe` 也在同一目录，便携版压缩包里两者都没有。
//!
//! 判断依据是文件而非进程权限，核心与配置程序因此必然得出同一结果——核心可能以
//! 管理员身份运行、配置程序不会，若各按自己能否写入来选目录，两边会各读一份配置。

use std::path::{Path, PathBuf};

/// 配置文件名。
pub const CONFIG_FILE_NAME: &str = "config.json";
/// 数据目录在 `%APPDATA%` 下的名字。
pub const USER_DIR_NAME: &str = "BossKey";
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

/// 用户目录 `%APPDATA%\BossKey`；取不到 `%APPDATA%` 时退回 exe 同目录。
pub fn user_data_dir() -> PathBuf {
    match std::env::var_os("APPDATA") {
        Some(appdata) if !appdata.is_empty() => PathBuf::from(appdata).join(USER_DIR_NAME),
        _ => exe_dir(),
    }
}

/// 程序目录里有没有安装痕迹。
///
/// 认两样东西：安装包放的 [`INSTALLED_MARKER`]，以及卸载程序 `unins*.exe`。
/// 后者是兜底——标记文件被误删时仍认得出是安装版，不至于把数据写回 `Program Files`。
/// 卸载程序的序号会随重复安装递增（`unins000` / `unins001`…），故按前缀匹配。
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
    let probe = dir.join(format!(".BossKey-write-probe-{}", std::process::id()));
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
///
/// 目标已有配置就不动它——那是当前在用的一份，旧文件不得覆盖，也不去删。
/// 删不掉（`Program Files` 下没有写权限、文件被占用）就留在原处：安装版只认用户目录，
/// 旧文件不会再被读到。
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

/// 定位数据目录。
///
/// `installed` 与 `portable_writable` 由调用方探测（生产走 [`is_installed`] 与
/// [`dir_writable`]），便于测试注入。
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
        // 用户目录也建不出来时无处可去。保存多半仍会失败，但错误里带得出路径。
        return located(program_dir.to_path_buf(), kind);
    }
    migrate_config(program_dir, user_dir);
    located(user_dir.to_path_buf(), kind)
}

/// 本次运行使用的数据目录。核心与配置程序共用，两边必须得出同一结果。
pub fn locate() -> DataDir {
    let program_dir = exe_dir();
    let installed = is_installed(&program_dir);
    // 安装版结果一样是用户目录，没必要再往程序目录里试写一次。
    let portable_writable = !installed && dir_writable(&program_dir);
    resolve_data_dir(&program_dir, &user_data_dir(), installed, portable_writable)
}

/// 本次运行使用的数据目录路径。
pub fn data_dir() -> PathBuf {
    locate().dir
}

/// 配置文件路径。
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
