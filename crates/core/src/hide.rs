use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use zonedeck_common::matching::{
    IgnoreMode, WindowResolution, is_config_image, is_ignored, match_process_rule,
    resolve_window_rule,
};
use zonedeck_common::{Config, NO_TITLE, Setting, WhitelistRule, WindowInfo, WindowRule};

use crate::effects::{Effects, PauseTarget};
use crate::platform::{Restore, WindowManager};
use crate::recovery::{MuteRecord, ProcRecord, Snapshot};

/// 一条隐藏记录。`title` 仅供日志，进程路径与映像名还用于白名单判定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub hwnd: i64,
    pub pid: u32,
    #[serde(default)]
    pub process_path: String,
    /// 映像名。路径查得到时可由它推出，查不到时（反作弊进程）只有这一项可用。
    #[serde(default)]
    pub process: String,
    #[serde(default)]
    pub title: String,
    /// 恢复时该怎么对待这个窗口，见 [`Restore`]。
    #[serde(default)]
    pub restore: Restore,
}

impl Target {
    /// 只有句柄与 PID 的记录。
    pub fn bare(hwnd: i64, pid: u32) -> Self {
        Self {
            hwnd,
            pid,
            process_path: String::new(),
            process: String::new(),
            title: String::new(),
            restore: Restore::default(),
        }
    }

    pub fn from_window(w: &WindowInfo) -> Self {
        Self {
            hwnd: w.hwnd,
            pid: w.pid,
            process_path: w.path.clone(),
            process: w.process.clone(),
            title: w.title.clone(),
            restore: Restore::default(),
        }
    }

    /// 可执行文件名（如 `WeChat.exe`）；路径与映像名都为空时返回空串。
    pub fn process_name(&self) -> &str {
        if !self.process.is_empty() {
            return &self.process;
        }
        std::path::Path::new(&self.process_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
    }

    /// 日志用的一行摘要：`进程名(hwnd=…, pid=…)`，不含窗口标题。
    pub fn describe(&self) -> String {
        let process = match self.process_name() {
            "" => "未知进程",
            name => name,
        };
        format!("{process}(hwnd={}, pid={})", self.hwnd, self.pid)
    }
}

/// 一条窗口规则的解析结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOutcome {
    Live,
    Reacquired,
    Missing,
    Regex(usize),
}

/// 依据窗口/进程规则解析出要隐藏的目标句柄集合，并对精确规则做追溯回填。
/// 返回值 `outcomes` 与 `config.window_rules` 一一对应。
pub fn resolve_targets(
    config: &mut Config,
    windows: &[WindowInfo],
    foreground: i64,
) -> (Vec<Target>, Vec<RuleOutcome>) {
    let mut result: Vec<Target> = Vec::new();
    let mut outcomes: Vec<RuleOutcome> = Vec::with_capacity(config.window_rules.len());

    for rule in &mut config.window_rules {
        let outcome = match resolve_window_rule(rule, windows) {
            WindowResolution::Live(w) => {
                rule.title = w.title.clone();
                result.push(Target::from_window(w));
                RuleOutcome::Live
            }
            WindowResolution::Reacquired(w) => {
                rule.hwnd = w.hwnd;
                rule.pid = w.pid;
                rule.title = w.title.clone();
                result.push(Target::from_window(w));
                RuleOutcome::Reacquired
            }
            WindowResolution::Missing => RuleOutcome::Missing,
            WindowResolution::Regex(hits) => {
                for w in &hits {
                    result.push(Target::from_window(w));
                }
                RuleOutcome::Regex(hits.len())
            }
        };
        outcomes.push(outcome);
    }

    for rule in &config.process_rules {
        for w in match_process_rule(rule, windows) {
            result.push(Target::from_window(w));
        }
    }

    if config.setting.hide_current && foreground != 0 {
        // 枚举不到的前台窗口（如工具窗口）只带句柄，PID 由 plan_hide 补查。
        match windows.iter().find(|w| w.hwnd == foreground) {
            Some(w) => result.push(Target::from_window(w)),
            None => result.push(Target::bare(foreground, 0)),
        }
    }

    // 配置窗口只在此刻可见时才纳入，本来就没开着的不去动它。
    if config.setting.hide_config_after_hide {
        for w in windows
            .iter()
            .filter(|w| w.visible && is_config_image(&w.process))
        {
            result.push(Target::from_window(w));
        }
    }

    let mut seen = HashSet::new();
    result.retain(|t| seen.insert(t.hwnd));
    // 「忽略隐藏」只打 Skip 标记、不移除。只带句柄的目标身份未知，
    // 由 `plan_hide` 补查路径后再判一次。
    for t in &mut result {
        if is_ignored(
            config.whitelist(),
            &t.process_path,
            t.process_name(),
            IgnoreMode::Hide,
        ) {
            t.restore = Restore::Skip;
        }
    }
    (result, outcomes)
}

/// 前台窗口对应的隐藏目标；只接受出现在枚举结果里且当前可见的顶层窗口。
pub fn foreground_target(windows: &[WindowInfo], foreground: i64) -> Option<Target> {
    if foreground == 0 {
        return None;
    }
    windows
        .iter()
        .find(|w| w.hwnd == foreground && w.visible)
        .map(Target::from_window)
}

/// 隐藏这一轮之后不会再有窗口留在桌面上的进程 PID 集合，冻结与静音共用这道门槛。
/// 仅当某进程的全部可见窗口都会被本程序藏起来（或它压根没有可见窗口）时才纳入。
pub fn dormant_pids(targets: &[Target], windows: &[WindowInfo]) -> Vec<u32> {
    // 只算会被本程序藏起来的窗口。
    let hidden: HashSet<i64> = targets
        .iter()
        .filter(|t| t.restore != Restore::Skip)
        .map(|t| t.hwnd)
        .collect();
    let mut pids: Vec<u32> = targets
        .iter()
        .map(|t| t.pid)
        .filter(|pid| *pid != 0)
        .collect();
    pids.sort_unstable();
    pids.dedup();

    pids.retain(|pid| {
        // all() 对空集恒为真，可见窗口一个不剩的进程同样纳入。
        windows
            .iter()
            .filter(|w| w.pid == *pid && w.visible)
            .all(|w| hidden.contains(&w.hwnd))
    });
    pids
}

/// 把一组根 PID 展开为「根 ∪ 全部后代」；`edges` 为 `(pid, 父 pid)`。
/// 可处理环与自指。返回排序去重后的列表。
pub fn expand_descendants(roots: &[u32], edges: &[(u32, u32)]) -> Vec<u32> {
    use std::collections::VecDeque;

    let mut children: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for &(pid, ppid) in edges {
        if pid != ppid {
            children.entry(ppid).or_default().push(pid);
        }
    }

    let mut visited: HashSet<u32> = HashSet::new();
    let mut queue: VecDeque<u32> = VecDeque::new();
    for &r in roots {
        if r != 0 && visited.insert(r) {
            queue.push_back(r);
        }
    }
    while let Some(pid) = queue.pop_front() {
        if let Some(kids) = children.get(&pid) {
            for &kid in kids {
                if kid != 0 && visited.insert(kid) {
                    queue.push_back(kid);
                }
            }
        }
    }

    let mut out: Vec<u32> = visited.into_iter().collect();
    out.sort_unstable();
    out
}

/// 把一组根 PID 扩展为「与它们映像名相同的全部进程」，不看亲缘关系。
/// `names` 为 `pid → 映像名`，按小写比对；查不到名字的根 PID 保留。
/// 返回排序去重后的列表。
pub fn expand_same_image(
    roots: &[u32],
    names: &std::collections::HashMap<u32, String>,
) -> Vec<u32> {
    let wanted: HashSet<String> = roots
        .iter()
        .filter_map(|pid| names.get(pid))
        .map(|name| name.to_ascii_lowercase())
        .collect();

    let mut out: Vec<u32> = names
        .iter()
        .filter(|(_, name)| wanted.contains(&name.to_ascii_lowercase()))
        .map(|(pid, _)| *pid)
        .collect();
    out.extend(roots.iter().copied());
    out.retain(|pid| *pid != 0);
    out.sort_unstable();
    out.dedup();
    out
}

/// 按白名单剔除不该冻结的 PID。
///
/// `names` 为 `pid → 映像名`，`paths` 为 `pid → 完整路径`（调用方按需填）。
/// 查不到名字的 PID 一律保留。须在范围展开
/// （[`expand_descendants`] / [`expand_same_image`]）之后调用。
pub fn filter_freeze_whitelist(
    pids: Vec<u32>,
    names: &std::collections::HashMap<u32, String>,
    paths: &std::collections::HashMap<u32, String>,
    whitelist: &[WhitelistRule],
) -> Vec<u32> {
    let mut pids = pids;
    pids.retain(|pid| {
        let name = names.get(pid).map(String::as_str).unwrap_or_default();
        let path = paths.get(pid).map(String::as_str).unwrap_or_default();
        !is_ignored(whitelist, path, name, IgnoreMode::Freeze)
    });
    pids
}

/// 一次隐藏的执行计划：由 [`HideController::plan_hide`] 算出、
/// [`HideController::commit_hide`] 原样执行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidePlan {
    /// 本次新增的隐藏目标；带 [`Restore::Skip`] 的不改动可见性，但副作用照常施加。
    pub fresh: Vec<Target>,
    /// 本次新增的静音进程。
    pub mute: Vec<MuteRecord>,
    /// 本次新增的冻结进程。
    pub freeze: Vec<ProcRecord>,
    /// 本次新增的效率模式进程。与冻结相互独立，两份名单可以不重合。
    pub efficiency: Vec<ProcRecord>,
    /// 本次要暂停媒体播放的目标；空表示不暂停。
    pub pause: Vec<PauseTarget>,
    /// 恢复时要不要把这些目标的媒体续播。隐藏那一刻就定下，
    /// 中途改设置不影响这一轮——恢复须还原隐藏时的意图。
    pub resume_media: bool,
    /// 本轮冻结方式；首轮跟随设置，之后沿用。
    pub enhanced: bool,
    /// 冻结后是否清空这些进程的工作集。
    pub trim: bool,
}

/// 收集一个暂停目标；PID 已经在列表里就跳过。
fn push_pause(list: &mut Vec<PauseTarget>, pid: u32, path: &str) {
    if pid == 0 || list.iter().any(|t| t.pid == pid) {
        return;
    }
    list.push(PauseTarget {
        pid,
        path: path.to_string(),
    });
}

/// [`HideController::show`] 的执行结果。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShowOutcome {
    /// 实际恢复显示的窗口数。
    pub shown: usize,
    /// 因句柄失效或已被其他窗口复用而跳过的记录数。
    pub stale: usize,
    /// 句柄失效后按「进程路径 + 标题」重新找回并显示的窗口数。
    pub refound: usize,
    /// 带 [`Restore::Skip`]、因而不予显示的记录数。
    pub skipped: usize,
}

/// [`HideController::adopt_from`] 的执行结果。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdoptOutcome {
    /// 核对无误、继续藏着的窗口数。
    pub kept: usize,
    /// 与记录对不上的窗口数：窗口没了、句柄被复用、换了进程，或已经被显示出来。
    pub mismatched: usize,
    /// 一个窗口都没接管成功时，顺带解除的进程副作用项数。
    pub released: usize,
}

/// 把标题变化同步进句柄匹配的精确窗口规则（仅内存，随下次配置保存落盘）。
/// `NO_TITLE` 不参与同步。返回是否有规则被更新。
pub fn sync_rule_titles(rules: &mut [WindowRule], hwnd: i64, title: &str) -> bool {
    if title == NO_TITLE {
        return false;
    }
    let mut changed = false;
    for rule in rules
        .iter_mut()
        .filter(|r| r.regex.is_none() && r.hwnd == hwnd && r.title != title)
    {
        rule.title = title.to_string();
        changed = true;
    }
    changed
}

pub struct HideController<W: WindowManager, E: Effects> {
    wm: W,
    effects: E,
    hidden: Vec<Target>,
    frozen: Vec<ProcRecord>,
    muted: Vec<MuteRecord>,
    /// 本轮暂停过媒体的目标，连同当时定下的续播意图。
    paused: Vec<PauseTarget>,
    resume_media: bool,
    efficiency: Vec<ProcRecord>,
    used_enhanced: bool,
}

impl<W: WindowManager, E: Effects> HideController<W, E> {
    pub fn new(wm: W, effects: E) -> Self {
        Self {
            wm,
            effects,
            hidden: Vec::new(),
            frozen: Vec::new(),
            muted: Vec::new(),
            paused: Vec::new(),
            resume_media: false,
            efficiency: Vec::new(),
            used_enhanced: false,
        }
    }

    /// 是否处于隐藏状态：至少藏起了一个窗口，或有进程被冻结 / 静音 / 降到效率模式。
    /// 带 [`Restore::Skip`] 的记录本程序没动过它的可见性，只为记账留着，不算数。
    pub fn is_hidden(&self) -> bool {
        self.hidden.iter().any(|t| t.restore != Restore::Skip)
            || !self.frozen.is_empty()
            || !self.muted.is_empty()
            || !self.efficiency.is_empty()
    }

    /// 是否还留着隐藏记录（含 [`Restore::Skip`]）。判断这一轮隐藏跑过没有用它，
    /// 而非 [`Self::is_hidden`]。
    pub fn tracks_any(&self) -> bool {
        !self.hidden.is_empty()
    }

    pub fn hidden_count(&self) -> usize {
        self.hidden.len()
    }

    pub fn enumerate(&self) -> Vec<WindowInfo> {
        self.wm.enumerate()
    }

    pub fn foreground(&self) -> i64 {
        self.wm.foreground()
    }

    pub fn window_title(&self, hwnd: i64) -> String {
        self.wm.window_title(hwnd)
    }

    pub fn tracks_window(&self, hwnd: i64) -> bool {
        self.hidden.iter().any(|t| t.hwnd == hwnd)
    }

    /// 移除句柄对应的隐藏记录，返回是否有记录被移除。
    pub fn forget_window(&mut self, hwnd: i64) -> bool {
        let before = self.hidden.len();
        self.hidden.retain(|t| t.hwnd != hwnd);
        self.hidden.len() != before
    }

    /// 同步隐藏记录里的窗口标题；`NO_TITLE` 不参与。返回是否有记录被更新。
    pub fn update_title(&mut self, hwnd: i64, title: &str) -> bool {
        if title == NO_TITLE {
            return false;
        }
        let mut changed = false;
        for t in self
            .hidden
            .iter_mut()
            .filter(|t| t.hwnd == hwnd && t.title != title)
        {
            t.title = title.to_string();
            changed = true;
        }
        changed
    }

    /// 计算一次隐藏的执行计划，不做任何窗口 / 副作用动作；顺带剪枝与 PID 补查。
    ///
    /// `freeze_pids` 须由调用方按作用范围展开并过好白名单（见
    /// [`filter_freeze_whitelist`]）；`mute_pids` 是未经展开的 [`dormant_pids`] 结果。
    /// 隐藏与静音的白名单过滤在此处完成，每个目标的 [`Restore`] 也在这里定下。
    ///
    /// 隐藏是累加的，`show` 时一并恢复。已在隐藏 / 静音 / 冻结集内的目标会被跳过。
    pub fn plan_hide(
        &mut self,
        setting: &Setting,
        targets: &[Target],
        freeze_pids: &[u32],
        efficiency_pids: &[u32],
        mute_pids: &[u32],
        whitelist: &[WhitelistRule],
    ) -> HidePlan {
        self.prune_stale();
        let known: HashSet<i64> = self.hidden.iter().map(|t| t.hwnd).collect();

        let mut fresh: Vec<Target> = Vec::new();
        for t in targets {
            if known.contains(&t.hwnd)
                || fresh.iter().any(|f| f.hwnd == t.hwnd)
                || !self.wm.is_window(t.hwnd)
            {
                continue;
            }
            let mut t = t.clone();
            if t.pid == 0 {
                t.pid = self.wm.window_pid(t.hwnd);
                if t.pid == 0 {
                    continue;
                }
            }
            // 任务栏与桌面带 WS_EX_TOOLWINDOW，不在枚举结果里，须先补上身份。
            if t.process_path.is_empty() && !whitelist.is_empty() {
                t.process_path = self.wm.process_path(t.pid);
            }
            if t.process.is_empty() && t.process_path.is_empty() {
                t.process = self.wm.process_name(t.pid);
            }
            if is_ignored(
                whitelist,
                &t.process_path,
                t.process_name(),
                IgnoreMode::Hide,
            ) {
                t.restore = Restore::Skip;
            }
            // 已判定为 Skip 的不再改写。
            if t.restore != Restore::Skip {
                t.restore = if !self.wm.is_visible(t.hwnd) {
                    Restore::Skip
                } else if setting.minimize_before_hide {
                    self.wm.restore_mode(t.hwnd)
                } else {
                    Restore::Show
                };
            }
            fresh.push(t);
        }

        let mut mute: Vec<MuteRecord> = Vec::new();
        if setting.mute_after_hide {
            for t in fresh.iter().filter(|t| mute_pids.contains(&t.pid)) {
                if !self.muted.iter().any(|r| r.pid == t.pid)
                    && !mute.iter().any(|r| r.pid == t.pid)
                    && !is_ignored(
                        whitelist,
                        &t.process_path,
                        t.process_name(),
                        IgnoreMode::Mute,
                    )
                {
                    mute.push(self.mute_record(t));
                }
            }
        }

        let mut freeze: Vec<ProcRecord> = Vec::new();
        if setting.freeze_after_hide {
            for pid in freeze_pids {
                if *pid != 0
                    && !self.frozen.iter().any(|r| r.pid == *pid)
                    && !freeze.iter().any(|r| r.pid == *pid)
                {
                    freeze.push(self.proc_record(*pid));
                }
            }
        }

        let mut efficiency: Vec<ProcRecord> = Vec::new();
        if setting.efficiency_after_hide {
            for pid in efficiency_pids {
                if *pid != 0
                    && !self.efficiency.iter().any(|r| r.pid == *pid)
                    && !efficiency.iter().any(|r| r.pid == *pid)
                {
                    efficiency.push(self.proc_record(*pid));
                }
            }
        }

        let mut pause: Vec<PauseTarget> = Vec::new();
        if setting.send_before_hide {
            for t in fresh.iter().filter(|t| t.restore != Restore::Skip) {
                let path = self.target_path(t);
                push_pause(&mut pause, t.pid, &path);
            }
            // 窗口原本就不可见、这一轮只做冻结的目标同样要停：进程一挂起，
            // 声音就卡在最后一帧了。
            for r in &freeze {
                let path = self.wm.process_path(r.pid);
                push_pause(&mut pause, r.pid, &path);
            }
        }

        HidePlan {
            resume_media: setting.send_before_hide && setting.resume_media_after_show,
            pause,
            // 解冻方式必须与冻结时一致。
            enhanced: if self.frozen.is_empty() {
                setting.enhanced_freeze
            } else {
                self.used_enhanced
            },
            trim: setting.trim_memory_after_freeze,
            fresh,
            mute,
            freeze,
            efficiency,
        }
    }

    /// 执行计划：同步隐藏窗口（必要时先 `SW_SHOWMINNOACTIVE` 再 `SW_HIDE`），
    /// 副作用经 [`Effects`] 施加。生产实现为异步队列，入队顺序即执行顺序。
    pub fn commit_hide(&mut self, plan: HidePlan) {
        // 暂停须排在冻结之前：进程一旦挂起，就再也处理不了暂停命令。
        if !plan.pause.is_empty() {
            self.effects.pause_media(&plan.pause);
            self.resume_media = plan.resume_media;
            for t in &plan.pause {
                if !self.paused.iter().any(|p| p.pid == t.pid) {
                    self.paused.push(t.clone());
                }
            }
        }

        for t in &plan.fresh {
            // 本程序没改过它的可见性，恢复时也不改。
            if t.restore == Restore::Skip {
                continue;
            }
            if t.restore.wants_minimize() {
                self.wm.minimize(t.hwnd);
            }
            self.wm.hide(t.hwnd);
        }

        for r in &plan.mute {
            self.effects.mute(r.pid, &r.path);
            self.muted.push(r.clone());
        }
        self.muted.sort_unstable_by_key(|r| r.pid);

        // 效率模式须排在冻结之前。
        for r in &plan.efficiency {
            self.effects.set_efficiency(r.pid);
            self.efficiency.push(*r);
        }
        self.efficiency.sort_unstable_by_key(|r| r.pid);

        self.used_enhanced = plan.enhanced;
        // 冻结前须等屏幕画完，整批只等一次。
        if !plan.freeze.is_empty() {
            self.effects.settle_before_freeze();
        }
        for r in &plan.freeze {
            self.effects.suspend(r.pid, plan.enhanced);
            // 清空工作集须排在挂起之后。
            if plan.trim {
                self.effects.trim_working_set(r.pid);
            }
            self.frozen.push(*r);
        }
        self.frozen.sort_unstable_by_key(|r| r.pid);

        self.hidden.extend(plan.fresh);
    }

    /// plan + commit 的便捷封装，不带白名单；供测试与不需要白名单的调用方使用。
    /// 静音门槛退化为「目标自己的 PID」，`power_pids` 同时用作冻结与效率模式的
    /// 候选集，实际施加哪一种由 `setting` 的两个开关决定。
    pub fn apply_hide(&mut self, setting: &Setting, targets: &[Target], power_pids: &[u32]) {
        let mute_pids: Vec<u32> = targets.iter().map(|t| t.pid).collect();
        let plan = self.plan_hide(setting, targets, power_pids, power_pids, &mute_pids, &[]);
        self.commit_hide(plan);
    }

    /// 剔除已不成立的隐藏记录：句柄失效、句柄被别的进程复用（PID 不符）、
    /// 或窗口已重新可见（被外部恢复显示或句柄复用）。
    fn prune_stale(&mut self) {
        let wm = &self.wm;
        self.hidden.retain(|t| {
            wm.is_window(t.hwnd) && wm.window_pid(t.hwnd) == t.pid && !wm.is_visible(t.hwnd)
        });
    }

    /// 记录进程身份；创建时刻查不到时记 0，恢复侧对 0 不做校验。
    fn proc_record(&self, pid: u32) -> ProcRecord {
        ProcRecord {
            pid,
            created_at: self.wm.process_start_time(pid),
        }
    }

    /// 目标的映像路径；记录里没带就现查一次。
    fn target_path(&self, t: &Target) -> String {
        if t.process_path.is_empty() {
            self.wm.process_path(t.pid)
        } else {
            t.process_path.clone()
        }
    }

    /// 记录静音目标的身份与映像路径；路径查不到时留空，那时只能按 PID 解除。
    fn mute_record(&self, t: &Target) -> MuteRecord {
        MuteRecord {
            pid: t.pid,
            created_at: self.wm.process_start_time(t.pid),
            path: self.target_path(t),
        }
    }

    /// 进程记录是否仍指向当初那个进程（PID 会被系统回收复用，须比对创建时刻）。
    fn proc_alive(&self, r: &ProcRecord) -> bool {
        let now = self.wm.process_start_time(r.pid);
        if now == 0 {
            return false;
        }
        r.created_at == 0 || now == r.created_at
    }

    /// 恢复全部隐藏窗口并撤销副作用；失效记录与 [`Restore::Skip`] 记录跳过，
    /// 分别计入 [`ShowOutcome`] 的 `stale` 与 `skipped`。
    pub fn show(&mut self) -> ShowOutcome {
        let mut outcome = ShowOutcome::default();

        let frozen = std::mem::take(&mut self.frozen);
        for r in &frozen {
            if self.proc_alive(r) {
                self.effects.resume(r.pid, self.used_enhanced);
            }
        }

        let hidden = std::mem::take(&mut self.hidden);
        let mut stale: Vec<&Target> = Vec::new();
        for t in &hidden {
            // 本程序没动过它的可见性，恢复时也不得把它弹出来。
            if t.restore == Restore::Skip {
                outcome.skipped += 1;
                continue;
            }
            // 句柄仍存活且仍属于当初的进程才恢复，避免弹出复用同一句柄的无关窗口。
            if self.wm.is_window(t.hwnd) && (t.pid == 0 || self.wm.window_pid(t.hwnd) == t.pid) {
                self.wm.restore(t.hwnd, t.restore);
                outcome.shown += 1;
            } else {
                outcome.stale += 1;
                stale.push(t);
            }
        }
        outcome.refound = self.refind_stale(&hidden, &stale);

        // 不做存活校验：静音波及的是同映像的全部会话，目标进程即使已经退出，
        // 同 exe 的其他会话仍得解除。误伤由会话级记账挡着，见 [`crate::audio::unmute`]。
        let muted = std::mem::take(&mut self.muted);
        for r in &muted {
            self.effects.unmute(r.pid, &r.path);
        }

        // 排在解冻之后：先让进程跑起来，再把它的调度待遇还回去。
        let efficiency = std::mem::take(&mut self.efficiency);
        for r in &efficiency {
            if self.proc_alive(r) {
                self.effects.clear_efficiency(r.pid);
            }
        }

        // 续播排在最末：挂起的进程收不到播放命令，得等它先跑起来。
        // 要不要续播看隐藏那一刻定下的意图，不看当前设置。
        let paused = std::mem::take(&mut self.paused);
        let resume = std::mem::replace(&mut self.resume_media, false);
        if !paused.is_empty() {
            if resume {
                self.effects.resume_media(&paused);
            } else {
                // 不续播就把记账丢掉，免得留到下一轮被误播。
                self.effects.forget_paused_media();
            }
        }
        outcome
    }

    /// 按「进程路径 + 标题」为失效记录找回窗口：只匹配当前不可见、且不在
    /// 本次隐藏集内的窗口，找到即恢复显示。返回找回数。
    fn refind_stale(&self, hidden: &[Target], stale: &[&Target]) -> usize {
        if stale
            .iter()
            .all(|t| t.process_path.is_empty() || t.title.is_empty() || t.title == NO_TITLE)
        {
            return 0;
        }
        let windows = self.wm.enumerate();
        let mut used: HashSet<i64> = hidden.iter().map(|t| t.hwnd).collect();
        let mut refound = 0;
        for t in stale {
            if t.process_path.is_empty() || t.title.is_empty() || t.title == NO_TITLE {
                continue;
            }
            if let Some(w) = windows.iter().find(|w| {
                !w.visible
                    && !used.contains(&w.hwnd)
                    && w.path == t.process_path
                    && w.title == t.title
            }) {
                used.insert(w.hwnd);
                self.wm.show(w.hwnd);
                refound += 1;
            }
        }
        refound
    }

    /// 释放指定进程的隐藏状态：显示窗口、解冻、取消静音，并从隐藏集移除。返回被释放的窗口数。
    pub fn release_pids(&mut self, pids: &[u32]) -> usize {
        if pids.is_empty() {
            return 0;
        }

        // 显示窗口前须先解冻。
        let (thaw, keep): (Vec<ProcRecord>, Vec<ProcRecord>) = self
            .frozen
            .iter()
            .copied()
            .partition(|r| pids.contains(&r.pid));
        for r in &thaw {
            if self.proc_alive(r) {
                self.effects.resume(r.pid, self.used_enhanced);
            }
        }
        self.frozen = keep;

        let (show, keep): (Vec<Target>, Vec<Target>) = self
            .hidden
            .iter()
            .cloned()
            .partition(|t| pids.contains(&t.pid));
        for t in &show {
            if t.restore != Restore::Skip && self.wm.is_window(t.hwnd) {
                self.wm.restore(t.hwnd, t.restore);
            }
        }
        self.hidden = keep;

        let (unmute, keep): (Vec<MuteRecord>, Vec<MuteRecord>) = self
            .muted
            .iter()
            .cloned()
            .partition(|r| pids.contains(&r.pid));
        for r in &unmute {
            self.effects.unmute(r.pid, &r.path);
        }
        self.muted = keep;

        let (restore_eco, keep): (Vec<ProcRecord>, Vec<ProcRecord>) = self
            .efficiency
            .iter()
            .copied()
            .partition(|r| pids.contains(&r.pid));
        for r in &restore_eco {
            if self.proc_alive(r) {
                self.effects.clear_efficiency(r.pid);
            }
        }
        self.efficiency = keep;

        show.len()
    }

    /// 恢复显示指定句柄（窗口恢复工具经 IPC 调用）。在隐藏记录里的窗口按
    /// 整进程释放（连同解冻 / 取消静音）；不在记录里的句柄直接恢复显示。
    /// 返回从记录中释放的窗口数。
    pub fn release_windows(&mut self, hwnds: &[i64]) -> usize {
        let known: HashSet<i64> = self.hidden.iter().map(|t| t.hwnd).collect();
        let mut pids: Vec<u32> = self
            .hidden
            .iter()
            .filter(|t| hwnds.contains(&t.hwnd))
            .map(|t| t.pid)
            .collect();
        pids.sort_unstable();
        pids.dedup();
        let released = self.release_pids(&pids);
        for &hwnd in hwnds {
            if !known.contains(&hwnd) && self.wm.is_window(hwnd) {
                self.wm.show(hwnd);
            }
        }
        released
    }

    /// 当前隐藏状态的快照，用于崩溃恢复落盘。
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            hidden: self.hidden.clone(),
            frozen: self.frozen.clone(),
            muted: self.muted.clone(),
            efficiency: self.efficiency.clone(),
            enhanced: self.used_enhanced,
            ..Default::default()
        }
    }

    /// 在当前状态上叠加执行计划后的快照，供意图先行落盘。
    pub fn planned_snapshot(&self, plan: &HidePlan) -> Snapshot {
        let mut snapshot = self.snapshot();
        snapshot.hidden.extend(plan.fresh.iter().cloned());
        snapshot.frozen.extend(plan.freeze.iter().copied());
        snapshot.muted.extend(plan.mute.iter().cloned());
        snapshot.efficiency.extend(plan.efficiency.iter().copied());
        snapshot.enhanced = plan.enhanced;
        snapshot
    }

    /// 从崩溃前的快照恢复；逐条身份校验由 [`Self::show`] 完成。
    pub fn restore_from(&mut self, snapshot: Snapshot) -> ShowOutcome {
        self.hidden = snapshot.hidden;
        self.frozen = snapshot.frozen;
        self.muted = snapshot.muted;
        self.efficiency = snapshot.efficiency;
        self.used_enhanced = snapshot.enhanced;
        self.show()
    }

    /// 接管上次异常退出留下的快照：逐条核对窗口还是不是当初那个、是不是仍藏着。
    /// 对得上的原样接着藏，**不把窗口弹出来**——上一轮核心是被外部干掉的，
    /// 用户的隐藏意图并没有变。对不上的只丢弃记录，绝不去动那个句柄现在指向的窗口。
    ///
    /// 一个窗口都没接管成功时，这轮隐藏已经失效，把还活着的进程解冻、取消静音、
    /// 清除效率模式，免得它们一直挂在那儿。
    pub fn adopt_from(&mut self, snapshot: Snapshot) -> AdoptOutcome {
        let mut outcome = AdoptOutcome::default();
        let mut kept = Vec::with_capacity(snapshot.hidden.len());
        for t in snapshot.hidden {
            if self.still_hidden_as_recorded(&t) {
                kept.push(t);
            } else {
                outcome.mismatched += 1;
            }
        }
        outcome.kept = kept.len();
        self.hidden = kept;
        self.used_enhanced = snapshot.enhanced;

        if outcome.kept > 0 {
            self.frozen = snapshot.frozen;
            self.muted = snapshot.muted;
            self.efficiency = snapshot.efficiency;
            return outcome;
        }

        for r in &snapshot.frozen {
            if self.proc_alive(r) {
                self.effects.resume(r.pid, snapshot.enhanced);
                outcome.released += 1;
            }
        }
        for r in &snapshot.muted {
            self.effects.unmute(r.pid, &r.path);
            outcome.released += 1;
        }
        // 排在解冻之后：先让进程跑起来，再把调度待遇还回去。
        for r in &snapshot.efficiency {
            if self.proc_alive(r) {
                self.effects.clear_efficiency(r.pid);
                outcome.released += 1;
            }
        }
        outcome
    }

    /// 这条记录指的还是当初那个窗口，而且仍然藏着吗。
    ///
    /// 标题不参与判定：游戏、聊天软件的标题随时在变，拿它比对会把正常情况误判成失配。
    fn still_hidden_as_recorded(&self, t: &Target) -> bool {
        if !self.wm.is_window(t.hwnd) {
            return false;
        }
        // 句柄会被系统回收复用，PID 对得上才算同一个窗口。
        if t.pid != 0 && self.wm.window_pid(t.hwnd) != t.pid {
            return false;
        }
        // PID 也会被回收，再比一次映像名。查不到映像名的（反作弊进程拒绝查询）
        // 不当作失配，否则每次重启都要误报一轮。
        if t.pid != 0 && !t.process.is_empty() {
            let now = self.wm.process_name(t.pid);
            if !now.is_empty() && !now.eq_ignore_ascii_case(&t.process) {
                return false;
            }
        }
        // 隐藏前就不可见的窗口本程序没动过它的可见性，这一条不作数。
        t.restore == Restore::Skip || !self.wm.is_visible(t.hwnd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    use zonedeck_common::{ProcessRule, WindowRule};

    fn win(title: &str, hwnd: i64, process: &str, path: &str) -> WindowInfo {
        WindowInfo::new(title, hwnd, process, hwnd as u32, path)
    }

    /// 与 `win` 相同，但可指定 PID。
    fn win_pid(title: &str, hwnd: i64, process: &str, pid: u32, path: &str) -> WindowInfo {
        WindowInfo::new(title, hwnd, process, pid, path)
    }

    fn wrule(title: &str, hwnd: i64, process: &str, path: &str) -> WindowRule {
        WindowRule::from_window(&win(title, hwnd, process, path))
    }

    /// 断言用：目标列表压缩为 (hwnd, pid) 序列。
    fn ids(targets: &[Target]) -> Vec<(i64, u32)> {
        targets.iter().map(|t| (t.hwnd, t.pid)).collect()
    }

    /// Mock 里进程创建时刻的默认值：`1000 + pid`。
    fn start_of(pid: u32) -> i64 {
        1000 + pid as i64
    }

    /// 复刻 agent 的隐藏编排。
    fn do_hide<W: WindowManager, E: Effects>(
        controller: &mut HideController<W, E>,
        config: &mut Config,
    ) {
        let windows = controller.enumerate();
        let foreground = controller.foreground();
        let (targets, _) = resolve_targets(config, &windows, foreground);
        let dormant = dormant_pids(&targets, &windows);
        let plan = controller.plan_hide(
            &config.setting,
            &targets,
            &dormant,
            &dormant,
            &dormant,
            config.whitelist(),
        );
        controller.commit_hide(plan);
    }

    #[test]
    fn resolves_window_rule_to_its_target() {
        let mut config = Config {
            window_rules: vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")],
            ..Default::default()
        };
        let windows = vec![
            win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];
        let (targets, outcomes) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(ids(&targets), vec![(10, 10)]);
        assert_eq!(outcomes, vec![RuleOutcome::Live]);
    }

    #[test]
    fn resolved_target_carries_path_and_title() {
        let mut config = Config {
            window_rules: vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")],
            ..Default::default()
        };
        let windows = vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];
        let (targets, _) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(targets[0].process_path, "C:\\WeChat.exe");
        assert_eq!(targets[0].title, "微信");
        assert_eq!(
            targets[0].describe(),
            "WeChat.exe(hwnd=10, pid=10)",
            "日志摘要不带窗口标题"
        );
    }

    #[test]
    fn window_rule_syncs_title_on_hide() {
        let mut config = Config {
            window_rules: vec![wrule("旧标题", 10, "app.exe", "C:\\app.exe")],
            ..Default::default()
        };
        let windows = vec![win("新标题", 10, "app.exe", "C:\\app.exe")];
        let (targets, _) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(ids(&targets), vec![(10, 10)]);
        assert_eq!(
            config.window_rules[0].title, "新标题",
            "隐藏时应同步最新标题"
        );
    }

    #[test]
    fn window_rule_reacquires_and_backfills_hwnd() {
        let mut config = Config {
            window_rules: vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")],
            ..Default::default()
        };
        let windows = vec![win("微信", 99, "WeChat.exe", "C:\\WeChat.exe")];
        let (targets, outcomes) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(ids(&targets), vec![(99, 99)]);
        assert_eq!(outcomes, vec![RuleOutcome::Reacquired]);
        assert_eq!(config.window_rules[0].hwnd, 99, "应回填新句柄");
    }

    #[test]
    fn window_rule_missing_when_window_gone() {
        let mut config = Config {
            window_rules: vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")],
            ..Default::default()
        };
        let windows = vec![win("记事本", 20, "notepad.exe", "C:\\notepad.exe")];
        let (targets, outcomes) = resolve_targets(&mut config, &windows, 0);
        assert!(targets.is_empty());
        assert_eq!(outcomes, vec![RuleOutcome::Missing]);
    }

    #[test]
    fn process_rule_hides_all_windows_of_same_executable() {
        let mut config = Config {
            process_rules: vec![ProcessRule::from_window(&win(
                "窗口一",
                10,
                "game.exe",
                "C:\\game.exe",
            ))],
            ..Default::default()
        };
        let windows = vec![
            win("窗口一", 10, "game.exe", "C:\\game.exe"),
            win("窗口二", 11, "game.exe", "C:\\game.exe"),
            win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];
        let (targets, _) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(ids(&targets), vec![(10, 10), (11, 11)]);
    }

    #[test]
    fn freeze_skipped_when_process_still_has_a_visible_window() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
            win_pid("文件传输助手", 11, "WeChat.exe", 500, "C:\\WeChat.exe"),
        ];
        let targets = vec![Target::bare(10, 500)];
        assert!(
            dormant_pids(&targets, &windows).is_empty(),
            "还有窗口开着时不应冻结"
        );
    }

    #[test]
    fn freeze_allowed_when_all_visible_windows_hidden() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
            win_pid("文件传输助手", 11, "WeChat.exe", 500, "C:\\WeChat.exe"),
        ];
        let targets = vec![Target::bare(10, 500), Target::bare(11, 500)];
        assert_eq!(dormant_pids(&targets, &windows), vec![500]);
    }

    #[test]
    fn freeze_ignores_already_invisible_windows() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
            win_pid("后台窗口", 11, "WeChat.exe", 500, "C:\\WeChat.exe").with_visibility(false),
        ];
        let targets = vec![Target::bare(10, 500)];
        assert_eq!(dormant_pids(&targets, &windows), vec![500]);
    }

    #[test]
    fn hiding_one_window_of_a_multi_window_app_does_not_freeze_it() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.freeze_after_hide = true;
        config.window_rules = vec![WindowRule::from_window(&win_pid(
            "微信",
            10,
            "WeChat.exe",
            500,
            "C:\\WeChat.exe",
        ))];

        let wm = MockWm::new(
            vec![
                win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
                win_pid("文件传输助手", 11, "WeChat.exe", 500, "C:\\WeChat.exe"),
            ],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        do_hide(&mut controller, &mut config);

        assert!(!controller.wm.is_visible(10), "目标窗口应被隐藏");
        assert!(controller.wm.is_visible(11), "同进程的另一个窗口应保持可见");
        assert!(
            controller.effects.suspends.borrow().is_empty(),
            "同进程还有窗口开着，不应冻结"
        );
    }

    #[test]
    fn release_pids_restores_hidden_and_frozen_windows_of_a_process() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.freeze_after_hide = true;
        config.setting.mute_after_hide = true;
        config.process_rules = vec![ProcessRule::from_window(&win_pid(
            "ZoneDeck 设置",
            10,
            "ZoneDeck.exe",
            700,
            "C:\\ZoneDeck.exe",
        ))];

        let wm = MockWm::new(
            vec![win_pid(
                "ZoneDeck 设置",
                10,
                "ZoneDeck.exe",
                700,
                "C:\\ZoneDeck.exe",
            )],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        do_hide(&mut controller, &mut config);
        assert!(!controller.wm.is_visible(10), "配置窗口已被隐藏");
        assert_eq!(*controller.effects.suspends.borrow(), vec![700], "已被冻结");

        assert_eq!(controller.release_pids(&[700]), 1);
        assert!(controller.wm.is_visible(10), "配置窗口应被放出来");
        assert_eq!(*controller.effects.resumes.borrow(), vec![700], "应先解冻");
        assert!(!controller.is_hidden(), "释放后已无隐藏内容");
    }

    #[test]
    fn hide_current_appends_foreground_and_dedups() {
        let mut config = Config::default();
        config.setting.hide_current = true;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];
        let windows = vec![
            win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];
        let (targets, _) = resolve_targets(&mut config.clone(), &windows, 20);
        assert_eq!(ids(&targets), vec![(10, 10), (20, 20)]);

        let (same, _) = resolve_targets(&mut config, &windows, 10);
        assert_eq!(ids(&same), vec![(10, 10)], "前台与已命中窗口相同应去重");
    }

    #[test]
    fn the_config_window_joins_the_hide_only_while_it_is_on_screen() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let open = vec![
            win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            win("ZoneDeck", 30, "config.exe", "C:\\ZoneDeck\\config.exe"),
        ];
        let (targets, _) = resolve_targets(&mut config.clone(), &open, 0);
        assert_eq!(ids(&targets), vec![(10, 10), (30, 30)], "开着就一起藏");

        let closed: Vec<WindowInfo> = open
            .iter()
            .map(|w| w.clone().with_visibility(w.hwnd == 10))
            .collect();
        let (targets, _) = resolve_targets(&mut config.clone(), &closed, 0);
        assert_eq!(ids(&targets), vec![(10, 10)], "没开着就不去动它");

        config.setting.hide_config_after_hide = false;
        let (targets, _) = resolve_targets(&mut config, &open, 0);
        assert_eq!(ids(&targets), vec![(10, 10)], "关掉开关后不再纳入");
    }

    struct MockWm {
        windows: Vec<WindowInfo>,
        foreground: i64,
        visible: RefCell<HashSet<i64>>,
        /// 仍然存在的句柄。
        exists: RefCell<HashSet<i64>>,
        /// 覆写某句柄当前所属的 PID（模拟句柄被别的窗口复用）。
        pid_overrides: RefCell<HashMap<i64, u32>>,
        /// 覆写某进程的创建时刻（模拟 PID 被回收复用；0 = 进程已退出）。
        start_overrides: RefCell<HashMap<u32, i64>>,
        /// 枚举不到的进程的映像路径（如任务栏 / 桌面所属的 explorer.exe）。
        paths: RefCell<HashMap<u32, String>>,
        /// 各窗口当前形态，缺省为 [`Restore::Normal`]。
        shape: RefCell<HashMap<i64, Restore>>,
        /// 按调用顺序记下的 minimize / restore 动作。
        moves: RefCell<Vec<String>>,
    }

    impl MockWm {
        fn new(windows: Vec<WindowInfo>, foreground: i64) -> Self {
            let handles: HashSet<i64> = windows.iter().map(|w| w.hwnd).collect();
            Self {
                windows,
                foreground,
                visible: RefCell::new(handles.clone()),
                exists: RefCell::new(handles),
                pid_overrides: RefCell::new(HashMap::new()),
                start_overrides: RefCell::new(HashMap::new()),
                paths: RefCell::new(HashMap::new()),
                shape: RefCell::new(HashMap::new()),
                moves: RefCell::new(Vec::new()),
            }
        }

        /// 登记一个枚举不到、但按 PID 能查出身份的窗口。
        fn add_unlisted(&self, hwnd: i64, pid: u32, path: &str) {
            self.exists.borrow_mut().insert(hwnd);
            self.visible.borrow_mut().insert(hwnd);
            self.pid_overrides.borrow_mut().insert(hwnd, pid);
            self.paths.borrow_mut().insert(pid, path.to_string());
        }

        fn destroy(&self, hwnd: i64) {
            self.visible.borrow_mut().remove(&hwnd);
            self.exists.borrow_mut().remove(&hwnd);
        }

        /// 模拟句柄被回收后分配给新窗口。
        fn revive(&self, hwnd: i64) {
            self.exists.borrow_mut().insert(hwnd);
            self.visible.borrow_mut().insert(hwnd);
        }

        /// 预置某窗口的形态（最大化 / 已最小化）。
        fn set_shape(&self, hwnd: i64, shape: Restore) {
            self.shape.borrow_mut().insert(hwnd, shape);
        }

        /// 当前形态，供断言恢复后的结果。
        fn shape_of(&self, hwnd: i64) -> Restore {
            self.restore_mode(hwnd)
        }
    }

    impl WindowManager for MockWm {
        fn enumerate(&self) -> Vec<WindowInfo> {
            let visible = self.visible.borrow();
            let exists = self.exists.borrow();
            self.windows
                .iter()
                .filter(|w| exists.contains(&w.hwnd))
                .map(|w| w.clone().with_visibility(visible.contains(&w.hwnd)))
                .collect()
        }
        fn hide(&self, hwnd: i64) {
            self.visible.borrow_mut().remove(&hwnd);
        }
        fn show(&self, hwnd: i64) {
            self.visible.borrow_mut().insert(hwnd);
        }
        fn minimize(&self, hwnd: i64) {
            self.moves.borrow_mut().push(format!("min:{hwnd}"));
            self.shape.borrow_mut().insert(hwnd, Restore::Minimized);
        }
        fn restore_mode(&self, hwnd: i64) -> Restore {
            self.shape
                .borrow()
                .get(&hwnd)
                .copied()
                .unwrap_or(Restore::Normal)
        }
        fn restore(&self, hwnd: i64, how: Restore) {
            if how == Restore::Skip {
                return;
            }
            self.moves.borrow_mut().push(format!("{how:?}:{hwnd}"));
            self.visible.borrow_mut().insert(hwnd);
            let shape = match how {
                Restore::Normal => Restore::Normal,
                Restore::Maximized => Restore::Maximized,
                Restore::Minimized => Restore::Minimized,
                Restore::Show | Restore::Skip => return,
            };
            self.shape.borrow_mut().insert(hwnd, shape);
        }
        fn is_visible(&self, hwnd: i64) -> bool {
            self.visible.borrow().contains(&hwnd)
        }
        fn foreground(&self) -> i64 {
            self.foreground
        }
        fn is_window(&self, hwnd: i64) -> bool {
            self.exists.borrow().contains(&hwnd)
        }
        fn window_pid(&self, hwnd: i64) -> u32 {
            if !self.is_window(hwnd) {
                return 0;
            }
            if let Some(pid) = self.pid_overrides.borrow().get(&hwnd) {
                return *pid;
            }
            self.windows
                .iter()
                .find(|w| w.hwnd == hwnd)
                .map(|w| w.pid)
                .unwrap_or(0)
        }
        fn window_title(&self, hwnd: i64) -> String {
            self.windows
                .iter()
                .find(|w| w.hwnd == hwnd)
                .map(|w| w.title.clone())
                .unwrap_or_else(|| zonedeck_common::NO_TITLE.to_string())
        }
        fn process_path(&self, pid: u32) -> String {
            self.paths
                .borrow()
                .get(&pid)
                .cloned()
                .or_else(|| {
                    self.windows
                        .iter()
                        .find(|w| w.pid == pid)
                        .map(|w| w.path.clone())
                })
                .unwrap_or_default()
        }
        fn process_start_time(&self, pid: u32) -> i64 {
            if pid == 0 {
                return 0;
            }
            self.start_overrides
                .borrow()
                .get(&pid)
                .copied()
                .unwrap_or(start_of(pid))
        }
    }

    #[derive(Default)]
    struct MockEffects {
        mutes: RefCell<Vec<(u32, bool)>>,
        suspends: RefCell<Vec<u32>>,
        resumes: RefCell<Vec<u32>>,
        trims: RefCell<Vec<u32>>,
        eco_on: RefCell<Vec<u32>>,
        eco_off: RefCell<Vec<u32>>,
        pauses: RefCell<u32>,
        /// 最后一次暂停请求的目标。
        pause_targets: RefCell<Vec<PauseTarget>>,
        /// 续播请求的目标；`None` 表示这一轮改为丢弃记账。
        resumed_media: RefCell<Vec<Option<Vec<PauseTarget>>>>,
        settles: RefCell<u32>,
        /// 冻结相关动作的调用顺序。
        order: RefCell<Vec<String>>,
    }

    impl Effects for MockEffects {
        fn mute(&self, pid: u32, _path: &str) {
            self.mutes.borrow_mut().push((pid, true));
        }
        fn unmute(&self, pid: u32, _path: &str) {
            self.mutes.borrow_mut().push((pid, false));
        }
        fn settle_before_freeze(&self) {
            *self.settles.borrow_mut() += 1;
        }
        fn suspend(&self, pid: u32, _enhanced: bool) {
            self.suspends.borrow_mut().push(pid);
            self.order.borrow_mut().push(format!("suspend:{pid}"));
        }
        fn resume(&self, pid: u32, _enhanced: bool) {
            self.resumes.borrow_mut().push(pid);
            self.order.borrow_mut().push(format!("resume:{pid}"));
        }
        fn trim_working_set(&self, pid: u32) {
            self.trims.borrow_mut().push(pid);
            self.order.borrow_mut().push(format!("trim:{pid}"));
        }
        fn set_efficiency(&self, pid: u32) {
            self.eco_on.borrow_mut().push(pid);
            self.order.borrow_mut().push(format!("eco_on:{pid}"));
        }
        fn clear_efficiency(&self, pid: u32) {
            self.eco_off.borrow_mut().push(pid);
            self.order.borrow_mut().push(format!("eco_off:{pid}"));
        }
        fn pause_media(&self, targets: &[PauseTarget]) {
            *self.pauses.borrow_mut() += 1;
            *self.pause_targets.borrow_mut() = targets.to_vec();
        }
        fn resume_media(&self, targets: &[PauseTarget]) {
            self.resumed_media.borrow_mut().push(Some(targets.to_vec()));
        }
        fn forget_paused_media(&self) {
            self.resumed_media.borrow_mut().push(None);
        }
    }

    #[test]
    fn toggle_applies_mute_and_freeze_then_restores() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;
        config.setting.send_before_hide = true;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
            ],
            10,
        );
        let mut controller = HideController::new(wm, MockEffects::default());

        do_hide(&mut controller, &mut config);
        assert!(controller.is_hidden());
        assert!(!controller.wm.is_visible(10), "微信应被隐藏");
        assert!(controller.wm.is_visible(20), "记事本不在绑定内应保持可见");
        assert_eq!(*controller.effects.mutes.borrow(), vec![(10, true)]);
        assert_eq!(*controller.effects.suspends.borrow(), vec![10]);
        assert_eq!(*controller.effects.pauses.borrow(), 1, "应发送一次暂停键");
        assert_eq!(
            *controller.effects.settles.borrow(),
            1,
            "冻结前须静置一次，等屏幕画完再让进程停摆"
        );

        let outcome = controller.show();
        assert_eq!(
            outcome,
            ShowOutcome {
                shown: 1,
                stale: 0,
                refound: 0,
                skipped: 0
            }
        );
        assert!(!controller.is_hidden());
        assert!(controller.wm.is_visible(10), "恢复后微信应可见");
        assert_eq!(*controller.effects.resumes.borrow(), vec![10], "应解冻");
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(10, true), (10, false)],
            "恢复后应取消静音"
        );
    }

    #[test]
    fn nothing_to_freeze_means_nothing_to_wait_for() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = false;
        config.setting.send_before_hide = true;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        do_hide(&mut controller, &mut config);

        assert_eq!(
            *controller.effects.settles.borrow(),
            0,
            "没有要冻结的进程就不该空等，静音不必为残影让路"
        );
        assert_eq!(*controller.effects.mutes.borrow(), vec![(10, true)]);
    }

    #[test]
    fn successive_hides_accumulate_and_restore_together() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            freeze_after_hide: false,
            ..Setting::default()
        };

        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
            ],
            10,
        );
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(&setting, &[Target::bare(10, 10)], &[]);
        controller.apply_hide(&setting, &[Target::bare(20, 20)], &[]);

        assert_eq!(
            controller.hidden_count(),
            2,
            "第二次隐藏不应挤掉第一次的窗口"
        );
        assert!(!controller.wm.is_visible(10) && !controller.wm.is_visible(20));

        controller.show();
        assert!(
            controller.wm.is_visible(10) && controller.wm.is_visible(20),
            "恢复应把累计隐藏的窗口一并放出来"
        );
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(10, true), (20, true), (10, false), (20, false)],
            "两个进程都应取消静音"
        );
    }

    #[test]
    fn re_hiding_the_same_window_does_not_repeat_effects() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            freeze_after_hide: true,
            ..Setting::default()
        };

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        let targets = [Target::bare(10, 10)];
        controller.apply_hide(&setting, &targets, &[10]);
        controller.apply_hide(&setting, &targets, &[10]);

        assert_eq!(controller.hidden_count(), 1, "同一窗口不应重复入集");
        assert_eq!(
            *controller.effects.suspends.borrow(),
            vec![10],
            "重复挂起会让解冻次数对不上，须跳过"
        );
        assert_eq!(*controller.effects.mutes.borrow(), vec![(10, true)]);

        controller.show();
        assert_eq!(
            *controller.effects.resumes.borrow(),
            vec![10],
            "一次解冻即可"
        );
        assert!(controller.wm.is_visible(10));
    }

    #[test]
    fn foreground_target_takes_the_visible_foreground_window() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
            win_pid("已隐藏", 11, "app.exe", 600, "C:\\app.exe").with_visibility(false),
        ];
        assert_eq!(
            foreground_target(&windows, 10).map(|t| (t.hwnd, t.pid)),
            Some((10, 500))
        );
        assert_eq!(foreground_target(&windows, 0), None, "无前台窗口");
        assert_eq!(
            foreground_target(&windows, 11),
            None,
            "已隐藏的窗口不该再次成为目标"
        );
        assert_eq!(
            foreground_target(&windows, 99),
            None,
            "枚举不到的窗口（如工具窗口）不作为目标"
        );
    }

    #[test]
    fn send_before_hide_fires_once_regardless_of_target_count() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = false;
        config.setting.freeze_after_hide = false;
        config.setting.send_before_hide = true;
        config.window_rules = vec![
            wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
            wrule("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
        ];

        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
            ],
            10,
        );
        let mut controller = HideController::new(wm, MockEffects::default());

        do_hide(&mut controller, &mut config);
        assert!(!controller.wm.is_visible(10) && !controller.wm.is_visible(20));
        assert_eq!(
            *controller.effects.pauses.borrow(),
            1,
            "多个目标也只应发送一次暂停键，否则「播放/暂停」会被切回播放"
        );
    }

    #[test]
    fn snapshot_reflects_hidden_state_and_clears_after_show() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        assert!(controller.snapshot().is_empty(), "初始快照应为空");

        do_hide(&mut controller, &mut config);
        let snapshot = controller.snapshot();
        assert_eq!(ids(&snapshot.hidden), vec![(10, 10)]);
        assert_eq!(
            snapshot.frozen,
            vec![ProcRecord {
                pid: 10,
                created_at: start_of(10)
            }],
            "冻结记录应带进程创建时刻"
        );
        assert_eq!(
            snapshot.muted,
            vec![MuteRecord {
                pid: 10,
                created_at: start_of(10),
                path: "C:\\WeChat.exe".into(),
            }],
            "静音记录须带映像路径，目标进程退出后才找得回同 exe 的会话"
        );

        controller.show();
        assert!(controller.snapshot().is_empty(), "显示后快照应清空");
    }

    #[test]
    fn planned_snapshot_matches_state_after_commit() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        let windows = controller.enumerate();
        let (targets, _) = resolve_targets(&mut config, &windows, 0);
        let dormant = dormant_pids(&targets, &windows);

        let plan =
            controller.plan_hide(&config.setting, &targets, &dormant, &dormant, &dormant, &[]);
        let planned = controller.planned_snapshot(&plan);
        controller.commit_hide(plan);
        let actual = controller.snapshot();

        assert_eq!(planned.hidden, actual.hidden);
        assert_eq!(planned.frozen, actual.frozen);
        assert_eq!(planned.muted, actual.muted);
        assert_eq!(planned.enhanced, actual.enhanced);
    }

    #[test]
    fn restore_from_snapshot_shows_windows_and_reverts_effects() {
        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        wm.hide(10);
        let mut controller = HideController::new(wm, MockEffects::default());

        let outcome = controller.restore_from(Snapshot {
            hidden: vec![Target::bare(10, 10)],
            frozen: vec![ProcRecord {
                pid: 10,
                created_at: start_of(10),
            }],
            muted: vec![MuteRecord {
                pid: 10,
                created_at: start_of(10),
                path: "C:\\WeChat.exe".into(),
            }],
            enhanced: false,
            ..Default::default()
        });

        assert_eq!(
            outcome,
            ShowOutcome {
                shown: 1,
                stale: 0,
                refound: 0,
                skipped: 0
            }
        );
        assert!(!controller.is_hidden(), "恢复完成后应回到未隐藏状态");
        assert!(controller.wm.is_visible(10), "崩溃前隐藏的窗口应被找回");
        assert_eq!(*controller.effects.resumes.borrow(), vec![10], "应解冻进程");
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(10, false)],
            "应取消静音"
        );
        assert!(controller.snapshot().is_empty());
    }

    /// 上一轮隐藏了 hwnd=10（WeChat.exe / pid 10），并冻结、静音了它的进程。
    fn crash_snapshot() -> Snapshot {
        Snapshot {
            hidden: vec![Target::from_window(&win(
                "微信",
                10,
                "WeChat.exe",
                "C:\\WeChat.exe",
            ))],
            frozen: vec![ProcRecord {
                pid: 10,
                created_at: start_of(10),
            }],
            muted: vec![MuteRecord {
                pid: 10,
                created_at: start_of(10),
                path: "C:\\WeChat.exe".into(),
            }],
            enhanced: false,
            ..Default::default()
        }
    }

    fn crashed_with(prepare: impl FnOnce(&MockWm)) -> HideController<MockWm, MockEffects> {
        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 0);
        // 上一轮核心是在窗口藏着的时候被干掉的。
        wm.hide(10);
        prepare(&wm);
        HideController::new(wm, MockEffects::default())
    }

    #[test]
    fn adopt_keeps_windows_that_are_still_hidden_as_recorded() {
        let mut controller = crashed_with(|_| {});
        let outcome = controller.adopt_from(crash_snapshot());

        assert_eq!(
            outcome,
            AdoptOutcome {
                kept: 1,
                mismatched: 0,
                released: 0
            }
        );
        assert!(!controller.wm.is_visible(10), "接管不得把窗口弹出来");
        assert!(controller.is_hidden(), "接管后仍是隐藏状态");
        assert!(
            controller.effects.resumes.borrow().is_empty(),
            "窗口还藏着，进程就该继续冻着"
        );
        assert!(controller.snapshot().hidden.len() == 1, "记录须留着供恢复");
    }

    #[test]
    fn adopt_drops_a_window_that_someone_already_brought_back() {
        // 解锁后游戏自己把窗口显示了出来：记录已经名不副实。
        let mut controller = crashed_with(|wm| wm.show(10));
        let outcome = controller.adopt_from(crash_snapshot());

        assert_eq!(outcome.kept, 0);
        assert_eq!(outcome.mismatched, 1);
        assert!(outcome.released > 0, "没有窗口要藏了，进程得放开");
        assert_eq!(*controller.effects.resumes.borrow(), vec![10], "应解冻");
        assert!(controller.wm.is_visible(10), "接管不得反过来把它藏回去");
    }

    #[test]
    fn adopt_drops_records_whose_window_is_gone_or_taken_over() {
        for (name, prepare) in [
            (
                "窗口已关闭",
                Box::new(|wm: &MockWm| wm.destroy(10)) as Box<dyn Fn(&MockWm)>,
            ),
            (
                "句柄被别的进程复用",
                Box::new(|wm: &MockWm| {
                    wm.pid_overrides.borrow_mut().insert(10, 999);
                }),
            ),
            (
                "PID 被回收给了别的程序",
                Box::new(|wm: &MockWm| {
                    wm.paths
                        .borrow_mut()
                        .insert(10, "C:\\Other.exe".to_string());
                }),
            ),
        ] {
            let mut controller = crashed_with(|wm| prepare(wm));
            let outcome = controller.adopt_from(crash_snapshot());
            assert_eq!(outcome.kept, 0, "{name}");
            assert_eq!(outcome.mismatched, 1, "{name}");
        }
    }

    #[test]
    fn adopt_does_not_judge_visibility_of_skip_records() {
        // 隐藏前就不可见的窗口本程序没动过它，可见与否都不算失配。
        let mut controller = crashed_with(|wm| wm.show(10));
        let mut snapshot = crash_snapshot();
        snapshot.hidden[0].restore = Restore::Skip;

        let outcome = controller.adopt_from(snapshot);
        assert_eq!(outcome.kept, 1);
        assert_eq!(outcome.mismatched, 0);
    }

    #[test]
    fn adopt_keeps_effects_as_long_as_one_window_survives() {
        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
            ],
            0,
        );
        wm.hide(10);
        wm.hide(20);
        // 其中一个被人显示了出来，另一个还藏着。
        wm.show(20);
        let mut controller = HideController::new(wm, MockEffects::default());

        let mut snapshot = crash_snapshot();
        snapshot.hidden.push(Target::from_window(&win(
            "记事本",
            20,
            "notepad.exe",
            "C:\\notepad.exe",
        )));

        let outcome = controller.adopt_from(snapshot);
        assert_eq!(outcome.kept, 1);
        assert_eq!(outcome.mismatched, 1);
        assert_eq!(outcome.released, 0, "还有窗口藏着，副作用不动");
        assert!(
            controller.effects.resumes.borrow().is_empty(),
            "不得解冻仍在隐藏的进程"
        );
    }

    #[test]
    fn dead_handle_is_pruned_and_reused_handle_can_hide_again() {
        let setting = Setting {
            hide_current: false,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(&setting, &[Target::bare(10, 10)], &[]);
        assert_eq!(controller.hidden_count(), 1);

        // 旧窗口销毁，句柄随后被新窗口复用。
        controller.wm.destroy(10);
        controller.wm.revive(10);

        controller.apply_hide(&setting, &[Target::bare(10, 10)], &[]);
        assert_eq!(
            controller.hidden_count(),
            1,
            "死记录应被剪掉，复用句柄的新窗口不应被误判为已隐藏"
        );
        assert!(!controller.wm.is_visible(10), "新窗口应真的被隐藏");
    }

    #[test]
    fn show_skips_destroyed_and_reused_handles() {
        let setting = Setting {
            hide_current: false,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![
                win("微信", 10, "WeChat.exe", "C:\\WeChat.exe"),
                win("记事本", 20, "notepad.exe", "C:\\notepad.exe"),
                win("画图", 30, "mspaint.exe", "C:\\mspaint.exe"),
            ],
            10,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(
            &setting,
            &[
                Target::bare(10, 10),
                Target::bare(20, 20),
                Target::bare(30, 30),
            ],
            &[],
        );

        controller.wm.destroy(10); // 窗口已退出。
        controller.wm.pid_overrides.borrow_mut().insert(20, 999); // 句柄被别的进程复用。

        let outcome = controller.show();
        assert_eq!(
            outcome,
            ShowOutcome {
                shown: 1,
                stale: 2,
                refound: 0,
                skipped: 0
            },
            "死句柄与被复用句柄都应跳过"
        );
        assert!(!controller.wm.is_visible(20), "被复用的句柄不得被弹出来");
        assert!(controller.wm.is_visible(30), "正常窗口应恢复");
    }

    #[test]
    fn resume_is_skipped_when_pid_was_recycled_or_process_exited() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: false,
            freeze_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![
                win_pid("A", 10, "a.exe", 100, "C:\\a.exe"),
                win_pid("B", 20, "b.exe", 200, "C:\\b.exe"),
            ],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(
            &setting,
            &[Target::bare(10, 100), Target::bare(20, 200)],
            &[100, 200],
        );

        // PID 100 被回收给了新进程（创建时刻变了）；PID 200 的进程已退出。
        controller
            .wm
            .start_overrides
            .borrow_mut()
            .insert(100, 9_999);
        controller.wm.start_overrides.borrow_mut().insert(200, 0);

        controller.show();
        assert!(
            controller.effects.resumes.borrow().is_empty(),
            "身份不符或已退出的进程不得解冻，避免干扰无关进程"
        );
    }

    /// 静音的目标在隐藏期间被关掉，恢复时仍要取消静音。静音波及的是同映像的
    /// 全部会话，它们未必随目标进程一起消失；误伤由会话级记账挡着，不靠存活校验。
    #[test]
    fn unmute_still_runs_after_the_target_process_exits() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe")],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[]);
        assert_eq!(*controller.effects.mutes.borrow(), vec![(100, true)]);

        // 进程已退出。
        controller.wm.start_overrides.borrow_mut().insert(100, 0);

        controller.show();
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(100, true), (100, false)],
            "目标进程已退出也要取消静音，同 exe 的其他会话还等着解除"
        );
    }

    /// 部分释放同样不看存活：道理与 [`unmute_still_runs_after_the_target_process_exits`] 一致。
    #[test]
    fn release_pids_unmutes_even_if_the_process_is_gone() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe")],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[]);
        controller.wm.start_overrides.borrow_mut().insert(100, 0);

        controller.release_pids(&[100]);
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(100, true), (100, false)]
        );
    }

    /// 暂停目标要带上被隐藏进程的身份，而不是只给一个「发不发」的布尔：
    /// 发之前得先确认是这些进程在出声，别把无关的后台播放器一起停了。
    #[test]
    fn pause_targets_carry_the_hidden_processes() {
        let setting = Setting {
            hide_current: false,
            send_before_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe")],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        let plan = controller.plan_hide(&setting, &[Target::bare(10, 100)], &[], &[], &[], &[]);

        assert_eq!(
            plan.pause,
            vec![PauseTarget {
                pid: 100,
                path: "C:\\WeChat.exe".into()
            }],
            "暂停目标须带映像路径，SMTC 靠它认出是哪个程序的媒体会话"
        );
    }

    /// 开了续播时，恢复要把暂停过的目标交回去续播。
    #[test]
    fn resume_media_plays_back_what_this_round_paused() {
        let setting = Setting {
            hide_current: false,
            send_before_hide: true,
            resume_media_after_show: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid(
                "网易云",
                10,
                "cloudmusic.exe",
                100,
                "C:\\cloudmusic.exe",
            )],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[]);
        assert_eq!(*controller.effects.pauses.borrow(), 1);

        controller.show();
        assert_eq!(
            *controller.effects.resumed_media.borrow(),
            vec![Some(vec![PauseTarget {
                pid: 100,
                path: "C:\\cloudmusic.exe".into()
            }])],
            "开了续播就该把这一轮暂停过的目标交回去"
        );
    }

    /// 默认不续播：恢复时只丢掉暂停记账，不替用户重新播放。
    #[test]
    fn without_resume_media_the_bookkeeping_is_dropped_instead() {
        let setting = Setting {
            hide_current: false,
            send_before_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid(
                "网易云",
                10,
                "cloudmusic.exe",
                100,
                "C:\\cloudmusic.exe",
            )],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        assert!(
            !setting.resume_media_after_show,
            "续播须默认关闭：替用户擅自恢复播放比不恢复更扰人"
        );
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[]);

        controller.show();
        assert_eq!(
            *controller.effects.resumed_media.borrow(),
            vec![None],
            "不续播时该丢掉记账，免得留到下一轮被误播"
        );
    }

    /// 续播与否在隐藏那一刻定下：隐藏期间把设置关掉，这一轮仍按当时的意图续播。
    #[test]
    fn resume_media_intent_is_fixed_at_hide_time() {
        let mut setting = Setting {
            hide_current: false,
            send_before_hide: true,
            resume_media_after_show: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid(
                "网易云",
                10,
                "cloudmusic.exe",
                100,
                "C:\\cloudmusic.exe",
            )],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[]);

        // 隐藏期间用户改了设置，本轮恢复不受影响。
        setting.resume_media_after_show = false;

        controller.show();
        assert!(
            matches!(
                controller.effects.resumed_media.borrow().as_slice(),
                [Some(_)]
            ),
            "恢复须还原隐藏时的意图，不看改过的设置"
        );
    }

    /// 窗口原本就不可见、这一轮只做冻结时照样要暂停：进程一挂起，声音就卡住了。
    #[test]
    fn freeze_only_round_still_pauses_media() {
        let setting = Setting {
            hide_current: false,
            send_before_hide: true,
            freeze_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid("微信", 10, "WeChat.exe", 100, "C:\\WeChat.exe")],
            0,
        );
        wm.hide(10); // 隐藏前就不可见，本程序不动它的可见性。
        let mut controller = HideController::new(wm, MockEffects::default());
        let plan = controller.plan_hide(&setting, &[Target::bare(10, 100)], &[100], &[], &[], &[]);

        assert!(
            plan.fresh.iter().all(|t| t.restore == Restore::Skip),
            "窗口本来就不可见，应记为 Skip"
        );
        assert_eq!(
            plan.pause,
            vec![PauseTarget {
                pid: 100,
                path: "C:\\WeChat.exe".into()
            }],
            "只做冻结的一轮同样要暂停"
        );
    }

    #[test]
    fn plan_fills_missing_pid_and_drops_unresolvable_targets() {
        let setting = Setting {
            hide_current: false,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid("画图", 30, "mspaint.exe", 33, "C:\\mspaint.exe")],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());

        // hwnd 40 不存在（is_window=false），hwnd 30 的 pid 现场补查为 33。
        controller.apply_hide(&setting, &[Target::bare(30, 0), Target::bare(40, 0)], &[]);
        assert_eq!(controller.hidden_count(), 1, "查无此窗的目标应被剔除");
        assert_eq!(
            controller.release_pids(&[33]),
            1,
            "补齐 PID 后 release_pids 应能按进程释放该窗口"
        );
    }

    #[test]
    fn forget_window_removes_record_and_update_title_syncs_it() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: false,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 0);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(
            &setting,
            &[Target::from_window(&win(
                "微信",
                10,
                "WeChat.exe",
                "C:\\WeChat.exe",
            ))],
            &[],
        );

        assert!(controller.tracks_window(10));
        assert!(controller.update_title(10, "微信 - 新会话"));
        assert!(
            !controller.update_title(10, "微信 - 新会话"),
            "标题未变不算更新"
        );
        assert!(
            !controller.update_title(10, zonedeck_common::NO_TITLE),
            "NO_TITLE 不参与同步"
        );
        assert_eq!(controller.snapshot().hidden[0].title, "微信 - 新会话");

        assert!(!controller.forget_window(99), "未记录的句柄无事发生");
        assert!(controller.forget_window(10));
        assert!(!controller.is_hidden(), "记录移除后不再处于隐藏态");
    }

    #[test]
    fn sync_rule_titles_updates_exact_rules_only() {
        let mut rules = vec![
            wrule("旧标题", 10, "app.exe", "C:\\app.exe"),
            WindowRule::from_regex("^项目"),
        ];
        assert!(sync_rule_titles(&mut rules, 10, "新标题"));
        assert_eq!(rules[0].title, "新标题");
        assert!(
            !sync_rule_titles(&mut rules, 10, "新标题"),
            "标题未变不算更新"
        );
        assert!(
            !sync_rule_titles(&mut rules, 99, "别的"),
            "句柄不匹配不更新"
        );
        assert!(
            !sync_rule_titles(&mut rules, 10, NO_TITLE),
            "NO_TITLE 不参与同步"
        );
    }

    #[test]
    fn show_refinds_stale_records_by_path_and_title() {
        let setting = Setting {
            hide_current: false,
            ..Setting::default()
        };
        // 99 是同进程同标题的新窗口，当前不可见。
        let wm = MockWm::new(
            vec![
                win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
                win_pid("微信", 99, "WeChat.exe", 600, "C:\\WeChat.exe"),
            ],
            0,
        );
        wm.hide(99);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(
            &setting,
            &[Target::from_window(&win_pid(
                "微信",
                10,
                "WeChat.exe",
                500,
                "C:\\WeChat.exe",
            ))],
            &[],
        );

        controller.wm.destroy(10);
        let outcome = controller.show();
        assert_eq!(
            outcome,
            ShowOutcome {
                shown: 0,
                stale: 1,
                refound: 1,
                skipped: 0
            }
        );
        assert!(
            controller.wm.is_visible(99),
            "应按进程路径 + 标题找回重建的窗口"
        );
    }

    #[test]
    fn refind_skips_visible_windows_and_records_without_info() {
        let setting = Setting {
            hide_current: false,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![
                win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
                win_pid("微信", 99, "WeChat.exe", 600, "C:\\WeChat.exe"),
            ],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        // bare 目标没有路径 / 标题信息，失效后无从找回。
        controller.apply_hide(&setting, &[Target::bare(10, 500)], &[]);
        controller.wm.destroy(10);
        let outcome = controller.show();
        assert_eq!(outcome.refound, 0, "无路径 / 标题信息的记录不找回");
        assert!(controller.wm.is_visible(99), "可见窗口不受影响");
    }

    /// 隐藏前就不可见的窗口：照常施加副作用，但恢复时不予显示。
    #[test]
    fn already_invisible_windows_get_effects_but_are_not_shown() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            ..Setting::default()
        };
        // 10 存在但已不可见（程序自己藏起来的），20 可见；两者都被规则命中。
        let wm = MockWm::new(
            vec![
                win_pid("Steam", 10, "steam.exe", 500, "C:\\steam.exe"),
                win_pid("记事本", 20, "notepad.exe", 600, "C:\\notepad.exe"),
            ],
            0,
        );
        wm.hide(10);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(
            &setting,
            &[Target::bare(10, 500), Target::bare(20, 600)],
            &[],
        );
        assert_eq!(
            controller.hidden_count(),
            2,
            "已不可见的窗口也要入集，否则恢复时没法撤销施加在它身上的副作用"
        );
        assert_eq!(
            controller.snapshot().hidden[0].restore,
            Restore::Skip,
            "它该被记为不予显示"
        );
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(500, true), (600, true)],
            "两个进程都该被静音——窗口藏着不代表进程没在放声音"
        );

        let outcome = controller.show();
        assert_eq!(
            outcome,
            ShowOutcome {
                shown: 1,
                stale: 0,
                refound: 0,
                skipped: 1
            },
            "只有记事本被显示，Steam 计入 skipped"
        );
        assert!(controller.wm.is_visible(20), "记事本应恢复");
        assert!(
            !controller.wm.is_visible(10),
            "Steam 自己藏起来的窗口不得被恢复弹出"
        );
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(500, true), (600, true), (500, false), (600, false)],
            "两个进程都该取消静音"
        );
    }

    /// 一个窗口都没藏成、也没施加副作用：这一轮什么都没发生，不算隐藏状态。
    #[test]
    fn a_round_that_touched_nothing_is_not_a_hidden_state() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: false,
            ..Setting::default()
        };
        // 窗口是目标程序自己藏起来的，本程序没动过它。
        let wm = MockWm::new(
            vec![win_pid("音乐", 10, "music.exe", 500, "C:\\music.exe")],
            0,
        );
        wm.hide(10);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(&setting, &[Target::bare(10, 500)], &[]);
        assert_eq!(
            controller.snapshot().hidden[0].restore,
            Restore::Skip,
            "前提：这条记录不改动可见性"
        );
        assert!(!controller.is_hidden(), "什么都没动就不该显示成隐藏状态");
        assert!(
            controller.tracks_any(),
            "记录照常入集，这一轮不该被自动隐藏再跑一遍"
        );
    }

    /// 窗口本来就藏着、只施加了副作用：仍算隐藏状态。
    #[test]
    fn effects_alone_still_count_as_a_hidden_state() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            freeze_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![win_pid("音乐", 10, "music.exe", 500, "C:\\music.exe")],
            0,
        );
        wm.hide(10);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(&setting, &[Target::bare(10, 500)], &[500]);
        assert_eq!(controller.snapshot().hidden[0].restore, Restore::Skip);
        assert_eq!(*controller.effects.suspends.borrow(), vec![500]);
        assert!(controller.is_hidden(), "进程还冻着，就得让用户能恢复");

        controller.show();
        assert!(!controller.is_hidden(), "副作用撤销后回到未隐藏状态");
    }

    /// 可见窗口一个不剩的进程同样该被冻结。
    #[test]
    fn process_with_only_invisible_windows_is_still_freezable() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe").with_visibility(false),
            win_pid("文件传输助手", 11, "WeChat.exe", 500, "C:\\WeChat.exe").with_visibility(false),
        ];
        let targets = vec![Target::bare(10, 500)];
        assert_eq!(
            dormant_pids(&targets, &windows),
            vec![500],
            "它本来就藏着，冻结正是用户要的"
        );
    }

    /// 还剩一个不在隐藏目标里的可见窗口时不冻结。
    #[test]
    fn one_visible_window_left_open_still_blocks_freezing() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe").with_visibility(false),
            win_pid("聊天窗口", 11, "WeChat.exe", 500, "C:\\WeChat.exe"),
        ];
        let targets = vec![Target::bare(10, 500)];
        assert!(
            dormant_pids(&targets, &windows).is_empty(),
            "还有窗口开着时不应冻结"
        );
    }

    // ---- 隐藏前先最小化 ----------------------------------------------------

    /// 便捷设置：只开「隐藏前先最小化」，其余副作用全关。
    fn minimizing_setting() -> Setting {
        Setting {
            hide_current: false,
            mute_after_hide: false,
            minimize_before_hide: true,
            ..Setting::default()
        }
    }

    #[test]
    fn normal_window_is_minimized_then_hidden_and_restored_to_normal() {
        let wm = MockWm::new(vec![win("记事本", 10, "notepad.exe", "C:\\notepad.exe")], 0);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(&minimizing_setting(), &[Target::bare(10, 10)], &[]);
        assert_eq!(
            controller.snapshot().hidden[0].restore,
            Restore::Normal,
            "普通窗口应记为 Normal"
        );
        assert!(!controller.wm.is_visible(10), "应已隐藏");
        assert_eq!(
            *controller.wm.moves.borrow(),
            vec!["min:10"],
            "隐藏前应先最小化"
        );

        controller.show();
        assert!(controller.wm.is_visible(10));
        assert_eq!(
            controller.wm.shape_of(10),
            Restore::Normal,
            "恢复应还原成普通大小"
        );
    }

    #[test]
    fn maximized_window_is_restored_to_maximized_not_normal() {
        let wm = MockWm::new(vec![win("浏览器", 10, "browser.exe", "C:\\browser.exe")], 0);
        wm.set_shape(10, Restore::Maximized);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(&minimizing_setting(), &[Target::bare(10, 10)], &[]);
        assert_eq!(controller.snapshot().hidden[0].restore, Restore::Maximized);

        controller.show();
        assert_eq!(
            controller.wm.shape_of(10),
            Restore::Maximized,
            "隐藏前是最大化，恢复就该是最大化，而不是被还原成普通大小"
        );
    }

    #[test]
    fn window_already_minimized_stays_minimized_after_restore() {
        let wm = MockWm::new(vec![win("音乐", 10, "music.exe", "C:\\music.exe")], 0);
        wm.set_shape(10, Restore::Minimized);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(&minimizing_setting(), &[Target::bare(10, 10)], &[]);
        assert_eq!(controller.snapshot().hidden[0].restore, Restore::Minimized);

        controller.show();
        assert!(controller.wm.is_visible(10), "应重新可见");
        assert_eq!(
            controller.wm.shape_of(10),
            Restore::Minimized,
            "本程序没最小化过它，恢复时也不该替它还原大小"
        );
    }

    /// 关掉该选项时不最小化，恢复只是显示出来。
    #[test]
    fn minimize_disabled_leaves_hide_and_show_untouched() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: false,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win("浏览器", 10, "browser.exe", "C:\\browser.exe")], 0);
        wm.set_shape(10, Restore::Maximized);
        let mut controller = HideController::new(wm, MockEffects::default());

        controller.apply_hide(&setting, &[Target::bare(10, 10)], &[]);
        assert_eq!(controller.snapshot().hidden[0].restore, Restore::Show);
        assert!(
            controller.wm.moves.borrow().is_empty(),
            "不该最小化任何窗口"
        );

        controller.show();
        assert!(controller.wm.is_visible(10));
        assert_eq!(
            controller.wm.shape_of(10),
            Restore::Maximized,
            "SW_SHOW 只改可见性，形态原样保留"
        );
    }

    /// 缺 `restore` 字段的恢复文件读进来回落 `Show`。
    #[test]
    fn legacy_target_without_restore_field_falls_back_to_show() {
        // MockWm 的 win() 约定 pid == hwnd。
        let t: Target =
            serde_json::from_str(r#"{"hwnd":10,"pid":10,"process_path":"","title":""}"#).unwrap();
        assert_eq!(t.restore, Restore::Show);

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 0);
        wm.hide(10);
        let mut controller = HideController::new(wm, MockEffects::default());
        let outcome = controller.restore_from(Snapshot {
            hidden: vec![t],
            ..Default::default()
        });
        assert_eq!(outcome.shown, 1, "旧快照的窗口照常恢复显示");
        assert!(controller.wm.is_visible(10));
    }

    // ---- 效率模式 ----------------------------------------------------------

    /// 效率模式独立于冻结：不开冻结也照样施加，恢复时撤销。
    #[test]
    fn efficiency_applies_without_freezing_and_is_cleared_on_show() {
        let setting = Setting {
            hide_current: false,
            freeze_after_hide: false,
            efficiency_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win_pid("A", 10, "a.exe", 100, "C:\\a.exe")], 0);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[100]);

        assert_eq!(*controller.effects.eco_on.borrow(), vec![100]);
        assert!(
            controller.effects.suspends.borrow().is_empty(),
            "没开冻结就不该冻结"
        );

        controller.show();
        assert_eq!(
            *controller.effects.eco_off.borrow(),
            vec![100],
            "恢复时应撤销效率模式"
        );
    }

    /// 冻结与效率模式同开时，效率模式排在冻结之前。
    #[test]
    fn efficiency_is_applied_before_freezing() {
        let setting = Setting {
            hide_current: false,
            freeze_after_hide: true,
            efficiency_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win_pid("A", 10, "a.exe", 100, "C:\\a.exe")], 0);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[100]);

        assert_eq!(
            *controller.effects.order.borrow(),
            vec!["eco_on:100", "suspend:100"],
        );

        controller.show();
        assert_eq!(
            *controller.effects.order.borrow(),
            vec!["eco_on:100", "suspend:100", "resume:100", "eco_off:100"],
            "恢复时先解冻再还调度待遇"
        );
    }

    #[test]
    fn efficiency_is_skipped_when_the_option_is_off() {
        let setting = Setting {
            hide_current: false,
            efficiency_after_hide: false,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win_pid("A", 10, "a.exe", 100, "C:\\a.exe")], 0);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[100]);

        assert!(controller.effects.eco_on.borrow().is_empty());
    }

    /// 快照要带上效率模式记录。
    #[test]
    fn efficiency_survives_the_recovery_snapshot() {
        let setting = Setting {
            hide_current: false,
            efficiency_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win_pid("A", 10, "a.exe", 100, "C:\\a.exe")], 0);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[100]);

        let snapshot = controller.snapshot();
        assert_eq!(
            snapshot
                .efficiency
                .iter()
                .map(|r| r.pid)
                .collect::<Vec<_>>(),
            vec![100]
        );
        assert!(!snapshot.is_empty(), "带效率模式记录的快照不算空");

        // 换一个控制器从快照恢复，应当把效率模式撤掉。
        let wm = MockWm::new(vec![win_pid("A", 10, "a.exe", 100, "C:\\a.exe")], 0);
        let mut fresh = HideController::new(wm, MockEffects::default());
        fresh.restore_from(snapshot);
        assert_eq!(*fresh.effects.eco_off.borrow(), vec![100]);
    }

    // ---- 降低内存占用 ------------------------------------------------------

    #[test]
    fn trimming_runs_once_per_frozen_process_right_after_suspend() {
        let setting = Setting {
            hide_current: false,
            freeze_after_hide: true,
            trim_memory_after_freeze: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![
                win_pid("A", 10, "a.exe", 100, "C:\\a.exe"),
                win_pid("B", 20, "b.exe", 200, "C:\\b.exe"),
            ],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(
            &setting,
            &[Target::bare(10, 100), Target::bare(20, 200)],
            &[100, 200],
        );

        assert_eq!(*controller.effects.trims.borrow(), vec![100, 200]);
        assert_eq!(
            *controller.effects.order.borrow(),
            vec!["suspend:100", "trim:100", "suspend:200", "trim:200"],
            "清空工作集必须紧跟在挂起之后：还在跑的进程会立刻把页读回来"
        );
    }

    #[test]
    fn trimming_is_skipped_when_the_option_is_off() {
        let setting = Setting {
            hide_current: false,
            freeze_after_hide: true,
            trim_memory_after_freeze: false,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win_pid("A", 10, "a.exe", 100, "C:\\a.exe")], 0);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[100]);

        assert_eq!(*controller.effects.suspends.borrow(), vec![100], "照常冻结");
        assert!(
            controller.effects.trims.borrow().is_empty(),
            "没开选项就不该清工作集"
        );
    }

    /// 不冻结就没有可清的对象。
    #[test]
    fn trimming_needs_something_frozen_to_act_on() {
        let setting = Setting {
            hide_current: false,
            freeze_after_hide: false,
            trim_memory_after_freeze: true,
            ..Setting::default()
        };
        let wm = MockWm::new(vec![win_pid("A", 10, "a.exe", 100, "C:\\a.exe")], 0);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(&setting, &[Target::bare(10, 100)], &[100]);

        assert!(controller.effects.suspends.borrow().is_empty());
        assert!(
            controller.effects.trims.borrow().is_empty(),
            "总开关没开时不该冻结，自然也没有可清的工作集"
        );
    }

    #[test]
    fn release_windows_frees_whole_process_and_shows_unknown_handles() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            freeze_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![
                win_pid("主窗口", 10, "game.exe", 500, "C:\\game.exe"),
                win_pid("子窗口", 11, "game.exe", 500, "C:\\game.exe"),
                win_pid("无关窗口", 30, "other.exe", 700, "C:\\other.exe"),
            ],
            0,
        );
        // 30 被别的途径隐藏，不在隐藏记录里。
        wm.hide(30);
        let mut controller = HideController::new(wm, MockEffects::default());
        controller.apply_hide(
            &setting,
            &[Target::bare(10, 500), Target::bare(11, 500)],
            &[500],
        );

        assert_eq!(controller.release_windows(&[10, 30]), 2);
        assert!(
            controller.wm.is_visible(10) && controller.wm.is_visible(11),
            "同进程的两个窗口应一起释放，避免解冻后仍有窗口藏着"
        );
        assert_eq!(*controller.effects.resumes.borrow(), vec![500], "应解冻");
        assert!(controller.wm.is_visible(30), "记录外的句柄应直接恢复显示");
        assert!(!controller.is_hidden());
    }

    #[test]
    fn expand_descendants_collects_multi_level_tree() {
        // 1 → 2 → 4, 1 → 3；另有无关的 9 → 10。
        let edges = [(2, 1), (3, 1), (4, 2), (10, 9)];
        let got = expand_descendants(&[1], &edges);
        assert_eq!(got, vec![1, 2, 3, 4], "应收集根及全部后代，不含无关分支");
    }

    #[test]
    fn expand_descendants_handles_cycles_and_self_reference() {
        // 自指 (0,0) 与环 1→2→1 都不能导致死循环。
        let edges = [(0, 0), (1, 2), (2, 1)];
        let got = expand_descendants(&[1], &edges);
        assert_eq!(got, vec![1, 2]);
    }

    #[test]
    fn expand_descendants_dedups_overlapping_roots_and_skips_zero() {
        let edges = [(2, 1), (3, 1)];
        // 根含 0（应跳过）与重叠子树。
        let got = expand_descendants(&[1, 2, 0], &edges);
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn expand_descendants_leaf_root_returns_itself() {
        let edges = [(2, 1)];
        assert_eq!(expand_descendants(&[2], &edges), vec![2]);
        assert!(expand_descendants(&[], &edges).is_empty());
    }

    /// 便捷构造：`pid → 映像名` 映射。
    fn names_of(pairs: &[(u32, &str)]) -> std::collections::HashMap<u32, String> {
        pairs
            .iter()
            .map(|(pid, name)| (*pid, name.to_string()))
            .collect()
    }

    #[test]
    fn expand_same_image_collects_every_instance_of_the_exe() {
        let names = names_of(&[
            (100, "chrome.exe"),
            (200, "chrome.exe"),
            (300, "notepad.exe"),
        ]);
        assert_eq!(
            expand_same_image(&[100], &names),
            vec![100, 200],
            "命中一个实例即拿下同名的全部实例，无关进程不受影响"
        );
    }

    /// Windows 文件名大小写不敏感，多开的实例大小写可能不一致。
    #[test]
    fn expand_same_image_is_case_insensitive() {
        let names = names_of(&[(100, "WeChat.exe"), (200, "wechat.EXE"), (300, "other.exe")]);
        assert_eq!(expand_same_image(&[100], &names), vec![100, 200]);
    }

    /// 查不到名字的根 PID 仍要保留。
    #[test]
    fn expand_same_image_keeps_roots_with_unknown_names() {
        let names = names_of(&[(100, "a.exe")]);
        assert_eq!(expand_same_image(&[100, 999], &names), vec![100, 999]);
        assert!(expand_same_image(&[], &names).is_empty());
        assert!(
            expand_same_image(&[0], &names).is_empty(),
            "PID 0 不是进程，应被剔除"
        );
    }

    /// 不看亲缘关系：同名实例全收，不同名的子进程不收。
    #[test]
    fn expand_same_image_ignores_parentage() {
        let names = names_of(&[
            (100, "game.exe"),
            (200, "game.exe"),
            (300, "GameHelper.exe"),
        ]);
        assert_eq!(
            expand_same_image(&[100], &names),
            vec![100, 200],
            "不同名的辅助进程要靠「目标进程及所有子进程」才收得到"
        );
    }

    #[test]
    fn disabled_effects_are_not_applied() {
        let mut config = Config::default();
        config.setting.hide_current = false;
        config.setting.mute_after_hide = false;
        config.setting.freeze_after_hide = false;
        config.setting.send_before_hide = false;
        config.window_rules = vec![wrule("微信", 10, "WeChat.exe", "C:\\WeChat.exe")];

        let wm = MockWm::new(vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")], 10);
        let mut controller = HideController::new(wm, MockEffects::default());

        do_hide(&mut controller, &mut config);
        assert!(controller.effects.mutes.borrow().is_empty());
        assert!(controller.effects.suspends.borrow().is_empty());
        assert_eq!(*controller.effects.pauses.borrow(), 0);
    }

    // ---- 白名单 ------------------------------------------------------------

    /// 一条按文件名匹配、只开指定模式的白名单条目。
    fn allow(process: &str, mode: IgnoreMode) -> WhitelistRule {
        let mut r = WhitelistRule::from_window(&win("", 0, process, &format!("C:\\{process}")));
        match mode {
            IgnoreMode::Hide => r.ignore_hide = true,
            IgnoreMode::Freeze => r.ignore_freeze = true,
            IgnoreMode::Mute => r.ignore_mute = true,
        }
        r
    }

    /// 忽略隐藏：命中的窗口留在目标表内，但带上 `Skip` 标记。
    #[test]
    fn ignored_windows_are_marked_skip_not_dropped() {
        let mut config = Config {
            window_rules: vec![
                wrule(
                    "资源管理器",
                    10,
                    "explorer.exe",
                    "C:\\Windows\\explorer.exe",
                ),
                wrule("微信", 20, "WeChat.exe", "C:\\WeChat.exe"),
            ],
            whitelist: Some(vec![allow("explorer.exe", IgnoreMode::Hide)]),
            ..Default::default()
        };
        let windows = vec![
            win(
                "资源管理器",
                10,
                "explorer.exe",
                "C:\\Windows\\explorer.exe",
            ),
            win("微信", 20, "WeChat.exe", "C:\\WeChat.exe"),
        ];
        let (targets, outcomes) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(ids(&targets), vec![(10, 10), (20, 20)], "两个都还在表内");
        assert_eq!(targets[0].restore, Restore::Skip, "资源管理器不该被隐藏");
        assert_eq!(targets[1].restore, Restore::Show, "微信照常隐藏");
        assert_eq!(
            outcomes,
            vec![RuleOutcome::Live, RuleOutcome::Live],
            "规则本身仍然解析成功"
        );
        assert_eq!(
            dormant_pids(&targets, &windows),
            vec![20],
            "资源管理器的窗口还开着，不该被冻结或静音"
        );
    }

    /// 勾了「忽略隐藏」且窗口还开着：不隐藏、不冻结、也不静音。
    #[test]
    fn ignore_hide_with_a_visible_window_suppresses_nothing_else_either() {
        let mut config = Config {
            window_rules: vec![
                wrule("音乐", 10, "music.exe", "C:\\music.exe"),
                wrule("微信", 20, "WeChat.exe", "C:\\WeChat.exe"),
            ],
            whitelist: Some(vec![allow("music.exe", IgnoreMode::Hide)]),
            ..Default::default()
        };
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;

        let wm = MockWm::new(
            vec![
                win("音乐", 10, "music.exe", "C:\\music.exe"),
                win("微信", 20, "WeChat.exe", "C:\\WeChat.exe"),
            ],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        do_hide(&mut controller, &mut config);

        assert!(controller.wm.is_visible(10), "音乐的窗口必须还开着");
        assert!(!controller.wm.is_visible(20), "微信照常隐藏");
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(20, true)],
            "窗口还在桌面上的程序不得被静音"
        );
        assert_eq!(
            *controller.effects.suspends.borrow(),
            vec![20],
            "窗口还在桌面上的程序不得被冻结"
        );
    }

    /// 勾了「忽略隐藏」但已无可见窗口：冻结与静音照常施加。
    #[test]
    fn ignore_hide_still_freezes_and_mutes_once_it_has_no_visible_window() {
        let mut config = Config {
            window_rules: vec![wrule("音乐", 10, "music.exe", "C:\\music.exe")],
            whitelist: Some(vec![allow("music.exe", IgnoreMode::Hide)]),
            ..Default::default()
        };
        config.setting.hide_current = false;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;

        let wm = MockWm::new(vec![win("音乐", 10, "music.exe", "C:\\music.exe")], 0);
        wm.hide(10); // 程序自己缩进了托盘
        let mut controller = HideController::new(wm, MockEffects::default());
        do_hide(&mut controller, &mut config);

        assert_eq!(*controller.effects.suspends.borrow(), vec![10], "应被冻结");
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(10, true)],
            "应被静音"
        );

        controller.show();
        assert!(
            !controller.wm.is_visible(10),
            "本程序没藏过它，恢复时也不得把它弹出来"
        );
        assert_eq!(*controller.effects.resumes.borrow(), vec![10], "应解冻");
    }

    /// 忽略隐藏也拦得住「同时隐藏当前活动窗口」。
    #[test]
    fn ignored_foreground_window_is_not_hidden() {
        let mut config = Config {
            whitelist: Some(vec![allow("explorer.exe", IgnoreMode::Hide)]),
            ..Default::default()
        };
        config.setting.hide_current = true;
        let windows = vec![win(
            "资源管理器",
            10,
            "explorer.exe",
            "C:\\Windows\\explorer.exe",
        )];
        let (targets, _) = resolve_targets(&mut config, &windows, 10);
        assert_eq!(
            targets.iter().map(|t| t.restore).collect::<Vec<_>>(),
            vec![Restore::Skip],
            "留在表内，但标记为不动它的可见性"
        );
        assert!(
            dormant_pids(&targets, &windows).is_empty(),
            "窗口还开着，冻结与静音都不该沾它"
        );
    }

    /// 目标全被「忽略隐藏」挡下：跑完一轮桌面纹丝不动，不该显示成隐藏状态。
    #[test]
    fn a_round_blocked_entirely_by_the_whitelist_is_not_a_hidden_state() {
        let mut config = Config {
            whitelist: Some(vec![allow("explorer.exe", IgnoreMode::Hide)]),
            ..Default::default()
        };
        config.setting.hide_current = true;
        config.setting.mute_after_hide = true;
        config.setting.freeze_after_hide = true;

        let wm = MockWm::new(
            vec![win_pid(
                "资源管理器",
                10,
                "explorer.exe",
                500,
                "C:\\Windows\\explorer.exe",
            )],
            10,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        do_hide(&mut controller, &mut config);

        assert!(controller.wm.is_visible(10), "窗口必须还在桌面上");
        assert!(controller.effects.suspends.borrow().is_empty());
        assert!(controller.effects.mutes.borrow().is_empty());
        assert!(
            !controller.is_hidden(),
            "桌面纹丝不动，托盘不该显示成隐藏状态"
        );
    }

    /// 任务栏与桌面不在枚举结果里，只有句柄，须由 `plan_hide` 补查路径后再判。
    #[test]
    fn whitelisted_taskbar_is_not_hidden_even_though_it_is_unlisted() {
        const TASKBAR: i64 = 777;
        const EXPLORER_PID: u32 = 15172;

        let mut config = Config {
            whitelist: Some(vec![allow("explorer.exe", IgnoreMode::Hide)]),
            ..Default::default()
        };
        config.setting.hide_current = true;

        let wm = MockWm::new(
            vec![win("微信", 10, "WeChat.exe", "C:\\WeChat.exe")],
            TASKBAR,
        );
        wm.add_unlisted(TASKBAR, EXPLORER_PID, "C:\\Windows\\explorer.exe");
        let mut controller = HideController::new(wm, MockEffects::default());

        let windows = controller.enumerate();
        assert!(
            !windows.iter().any(|w| w.hwnd == TASKBAR),
            "前提：任务栏不在枚举结果里"
        );
        let (targets, _) = resolve_targets(&mut config, &windows, TASKBAR);
        assert_eq!(
            targets.iter().map(|t| t.hwnd).collect::<Vec<_>>(),
            vec![TASKBAR],
            "此时只有句柄，resolve_targets 判不出它属于 explorer"
        );

        let plan =
            controller.plan_hide(&config.setting, &targets, &[], &[], &[], config.whitelist());
        assert_eq!(
            plan.fresh.iter().map(|t| t.restore).collect::<Vec<_>>(),
            vec![Restore::Skip],
            "补查路径后应被判为不动它的可见性"
        );
        controller.commit_hide(plan);
        assert!(controller.wm.is_visible(TASKBAR), "任务栏必须还在");
    }

    /// 没被白名单的枚举外窗口仍然照常隐藏。
    #[test]
    fn unlisted_foreground_window_is_still_hidden_without_a_whitelist_entry() {
        const TASKBAR: i64 = 777;

        let mut config = Config {
            whitelist: Some(vec![allow("别的.exe", IgnoreMode::Hide)]),
            ..Default::default()
        };
        config.setting.hide_current = true;

        let wm = MockWm::new(Vec::new(), TASKBAR);
        wm.add_unlisted(TASKBAR, 15172, "C:\\Windows\\explorer.exe");
        let mut controller = HideController::new(wm, MockEffects::default());

        let (targets, _) = resolve_targets(&mut config, &controller.enumerate(), TASKBAR);
        let plan =
            controller.plan_hide(&config.setting, &targets, &[], &[], &[], config.whitelist());
        assert_eq!(plan.fresh.len(), 1);
        assert_eq!(plan.fresh[0].restore, Restore::Show);
        controller.commit_hide(plan);
        assert!(!controller.wm.is_visible(TASKBAR));
    }

    /// 恢复工具的手动隐藏传空白名单，不受白名单影响。
    #[test]
    fn manual_hide_bypasses_the_whitelist() {
        const TASKBAR: i64 = 777;

        let setting = Setting {
            hide_current: false,
            ..Setting::default()
        };
        let wm = MockWm::new(Vec::new(), 0);
        wm.add_unlisted(TASKBAR, 15172, "C:\\Windows\\explorer.exe");
        let mut controller = HideController::new(wm, MockEffects::default());

        let plan = controller.plan_hide(&setting, &[Target::bare(TASKBAR, 0)], &[], &[], &[], &[]);
        assert_eq!(plan.fresh.len(), 1, "空白名单不拦任何窗口");
        assert_eq!(plan.fresh[0].restore, Restore::Show);
    }

    /// 不在 `mute_pids` 里的目标照常隐藏，但不静音。
    #[test]
    fn mute_is_gated_by_the_same_dormancy_check_as_freezing() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            ..Setting::default()
        };
        let wm = MockWm::new(
            vec![
                win_pid("A", 10, "a.exe", 100, "C:\\a.exe"),
                win_pid("B", 20, "b.exe", 200, "C:\\b.exe"),
            ],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());

        // 只有 100 过了门槛。
        let targets = [Target::bare(10, 100), Target::bare(20, 200)];
        let plan = controller.plan_hide(&setting, &targets, &[], &[], &[100], &[]);
        controller.commit_hide(plan);

        assert_eq!(controller.hidden_count(), 2, "两个窗口都照常隐藏");
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(100, true)],
            "没过门槛的进程不得被静音"
        );
    }

    /// 忽略静音：窗口照常隐藏，只是不静音。
    #[test]
    fn ignored_process_is_hidden_but_not_muted() {
        let mut config = Config {
            window_rules: vec![
                wrule("音乐", 10, "music.exe", "C:\\music.exe"),
                wrule("微信", 20, "WeChat.exe", "C:\\WeChat.exe"),
            ],
            whitelist: Some(vec![allow("music.exe", IgnoreMode::Mute)]),
            ..Default::default()
        };
        config.setting.mute_after_hide = true;
        config.setting.hide_current = false;

        let wm = MockWm::new(
            vec![
                win("音乐", 10, "music.exe", "C:\\music.exe"),
                win("微信", 20, "WeChat.exe", "C:\\WeChat.exe"),
            ],
            0,
        );
        let mut controller = HideController::new(wm, MockEffects::default());
        do_hide(&mut controller, &mut config);

        assert!(
            !controller.wm.is_visible(10) && !controller.wm.is_visible(20),
            "忽略静音不影响隐藏本身"
        );
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(20, true)],
            "只有未被白名单命中的进程被静音"
        );
    }

    /// 冻结白名单按映像名剔除；查不到名字的 PID 保留。
    #[test]
    fn freeze_whitelist_drops_matching_pids_by_image_name() {
        let names = std::collections::HashMap::from([
            (100, "explorer.exe".to_string()),
            (200, "WeChat.exe".to_string()),
        ]);
        let got = filter_freeze_whitelist(
            vec![100, 200, 300],
            &names,
            &std::collections::HashMap::new(),
            &[allow("Explorer.EXE", IgnoreMode::Freeze)],
        );
        assert_eq!(got, vec![200, 300], "大小写不敏感；未知 PID 保留");
    }

    /// 展开子进程树后，核心必须被内置保护挡下；白名单为空时同样生效。
    /// 配置程序不在保护之列，跟着一起冻。
    #[test]
    fn freeze_whitelist_protects_the_core_inside_an_expanded_tree() {
        // explorer(100) → 核心(200) → 配置程序(300)，另有普通子进程 400。
        let edges = [(200, 100), (300, 200), (400, 100)];
        let expanded = expand_descendants(&[100], &edges);
        assert_eq!(expanded, vec![100, 200, 300, 400], "先展开整棵树");

        let names = std::collections::HashMap::from([
            (100, "explorer.exe".to_string()),
            (200, "ZoneDeck.exe".to_string()),
            (300, "config.exe".to_string()),
            (400, "helper.exe".to_string()),
        ]);
        let got = filter_freeze_whitelist(expanded, &names, &std::collections::HashMap::new(), &[]);
        assert_eq!(got, vec![100, 300, 400], "只有核心留下");
    }

    #[test]
    fn freeze_whitelist_can_match_by_full_path() {
        let names = std::collections::HashMap::from([
            (100, "a.exe".to_string()),
            (200, "a.exe".to_string()),
        ]);
        let paths = std::collections::HashMap::from([
            (100, "C:\\Games\\a.exe".to_string()),
            (200, "D:\\别处\\a.exe".to_string()),
        ]);
        let mut rule = allow("a.exe", IgnoreMode::Freeze);
        rule.by_name = false;
        rule.path = "C:\\Games\\a.exe".to_string();

        let got = filter_freeze_whitelist(vec![100, 200], &names, &paths, &[rule]);
        assert_eq!(got, vec![200], "按路径匹配只放过指定位置的那一个");
    }

    #[test]
    fn target_process_name_falls_back_to_empty() {
        let t = Target::from_window(&win("微信", 10, "WeChat.exe", "C:\\a\\WeChat.exe"));
        assert_eq!(t.process_name(), "WeChat.exe");
        assert_eq!(Target::bare(1, 2).process_name(), "");
        assert_eq!(
            Target::bare(1, 2).describe(),
            "未知进程(hwnd=1, pid=2)",
            "无路径时日志摘要仍要可读"
        );
    }

    #[test]
    fn target_keeps_the_image_name_when_the_path_is_unavailable() {
        // 反作弊进程拒绝 OpenProcess，只有映像名可查。
        let t = Target::from_window(&win("魔兽世界", 10, "Wow.exe", ""));
        assert_eq!(t.process_path, "");
        assert_eq!(t.process_name(), "Wow.exe");
        assert_eq!(t.describe(), "Wow.exe(hwnd=10, pid=10)");
    }

    #[test]
    fn hide_whitelist_can_skip_a_process_known_only_by_name() {
        let mut config = Config {
            process_rules: vec![ProcessRule::from_window(&win(
                "魔兽世界",
                10,
                "Wow.exe",
                "",
            ))],
            whitelist: Some(vec![allow("Wow.exe", IgnoreMode::Hide)]),
            ..Default::default()
        };
        let windows = vec![win("魔兽世界", 10, "Wow.exe", "")];
        let (targets, _) = resolve_targets(&mut config, &windows, 0);
        assert_eq!(targets.len(), 1);
        assert_eq!(
            targets[0].restore,
            Restore::Skip,
            "查不到路径时白名单仍应按映像名放过它"
        );
    }
}
