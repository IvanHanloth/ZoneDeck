use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use zonedeck_common::matching::{
    IgnoreMode, WindowResolution, is_ignored, match_process_rule, resolve_window_rule,
};
use zonedeck_common::{Config, NO_TITLE, Setting, WhitelistRule, WindowInfo, WindowRule};

use crate::effects::Effects;
use crate::platform::WindowManager;
use crate::recovery::{ProcRecord, Snapshot};

/// 一条隐藏记录。`process_path` / `title` 仅供日志；旧恢复文件缺省为空串。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub hwnd: i64,
    pub pid: u32,
    #[serde(default)]
    pub process_path: String,
    #[serde(default)]
    pub title: String,
}

impl Target {
    /// 只有句柄与 PID 的记录。
    pub fn bare(hwnd: i64, pid: u32) -> Self {
        Self {
            hwnd,
            pid,
            process_path: String::new(),
            title: String::new(),
        }
    }

    pub fn from_window(w: &WindowInfo) -> Self {
        Self {
            hwnd: w.hwnd,
            pid: w.pid,
            process_path: w.path.clone(),
            title: w.title.clone(),
        }
    }

    /// 可执行文件名（如 `WeChat.exe`）；路径为空时返回空串。
    pub fn process_name(&self) -> &str {
        std::path::Path::new(&self.process_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
    }

    /// 日志用的一行摘要：`进程名(hwnd=…, pid=…)`。
    /// 不含窗口标题——标题属隐私内容，不写入日志。
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
        // 前台窗口可能不在枚举结果里（如工具窗口）；缺失的 PID 由 plan_hide 补查。
        match windows.iter().find(|w| w.hwnd == foreground) {
            Some(w) => result.push(Target::from_window(w)),
            None => result.push(Target::bare(foreground, 0)),
        }
    }

    let mut seen = HashSet::new();
    result.retain(|t| seen.insert(t.hwnd));
    // 白名单过滤放在最后：上面的循环持着 config.window_rules 的可变借用。
    // 这里只拦得住枚举得到的窗口——只有句柄的目标（`Target::bare`）身份未知，
    // 由 `HideController::plan_hide` 补查路径后再过一次白名单。
    result.retain(|t| {
        !is_ignored(
            config.whitelist(),
            &t.process_path,
            t.process_name(),
            IgnoreMode::Hide,
        )
    });
    (result, outcomes)
}

/// 前台窗口对应的隐藏目标。只接受出现在枚举结果里且当前可见的顶层窗口：
/// 工具窗口不在枚举结果内，已被隐藏的窗口 `visible` 为假。
pub fn foreground_target(windows: &[WindowInfo], foreground: i64) -> Option<Target> {
    if foreground == 0 {
        return None;
    }
    windows
        .iter()
        .find(|w| w.hwnd == foreground && w.visible)
        .map(Target::from_window)
}

/// 可以安全冻结的进程 PID 集合：仅当某进程的全部可见窗口都在隐藏目标里时才纳入。
pub fn freezable_pids(targets: &[Target], windows: &[WindowInfo]) -> Vec<u32> {
    let hidden: HashSet<i64> = targets.iter().map(|t| t.hwnd).collect();
    let mut pids: Vec<u32> = targets
        .iter()
        .map(|t| t.pid)
        .filter(|pid| *pid != 0)
        .collect();
    pids.sort_unstable();
    pids.dedup();

    pids.retain(|pid| {
        let mut visible = windows
            .iter()
            .filter(|w| w.pid == *pid && w.visible)
            .peekable();
        // all() 对空集恒为真，故须先确认存在可见窗口。
        visible.peek().is_some() && visible.all(|w| hidden.contains(&w.hwnd))
    });
    pids
}

/// 把一组根 PID 展开为「根 ∪ 全部后代」；`edges` 为 `(pid, 父 pid)`。
/// visited 兼作防环与自指保护（pid 0 的父是 0）。返回排序去重后的列表。
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

/// 按白名单剔除不该冻结的 PID。
///
/// `names` 为 `pid → 映像名`（一次进程快照即可得），`paths` 为 `pid → 完整路径`
/// （逐 PID 查开销大，调用方仅在白名单确有按路径的条目时才填）。查不到名字的 PID
/// 一律**保留**：拿不到身份就无从判定，此处不擅自放行。
///
/// 必须在 [`expand_descendants`] **之后**调用：被挡下的往往正是 explorer 子树里的
/// ZoneDeck 自己，展开前它根本不在集合里。
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
/// [`HideController::commit_hide`] 原样执行，两段之间由调用方落盘意图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidePlan {
    /// 本次新增的隐藏目标（已剔除死句柄 / 不可见项 / 已隐藏项 / 查不到 PID 的项）。
    pub fresh: Vec<Target>,
    /// 本次新增的静音进程。
    pub mute: Vec<ProcRecord>,
    /// 本次新增的冻结进程。
    pub freeze: Vec<ProcRecord>,
    /// 是否发送媒体暂停键。
    pub send_pause: bool,
    /// 本轮冻结方式；首轮跟随设置，之后沿用。
    pub enhanced: bool,
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
    muted: Vec<ProcRecord>,
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
            used_enhanced: false,
        }
    }

    pub fn is_hidden(&self) -> bool {
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

    /// 计算一次隐藏的执行计划，不做任何窗口 / 副作用动作；顺带完成隐藏集剪枝
    /// 与 PID 补查（仍查不到的目标剔除）。`freeze_pids` 仅在 `freeze_after_hide`
    /// 开启时生效，且须由调用方预先过好白名单（见 [`filter_freeze_whitelist`]）；
    /// 隐藏与静音的白名单过滤在此处完成。
    ///
    /// 隐藏在这里再过一次白名单（`resolve_targets` 已过过一次）是必要的：只带句柄的
    /// 目标（`hide_current` 命中枚举不到的前台窗口）当时无从判定身份，须先补查路径。
    /// 静音则是因为静音集由 `fresh` 现算，外部拦不住。
    ///
    /// 只有「由本程序从可见变为不可见」的窗口才进入隐藏集，恢复即逆转这次改变。
    /// 隐藏是累加的，`show` 时一并恢复。已在隐藏 / 静音 / 冻结集内的目标会被
    /// 跳过——挂起是计数式的，重复施加会让解冻次数对不上。
    pub fn plan_hide(
        &mut self,
        setting: &Setting,
        targets: &[Target],
        freeze_pids: &[u32],
        whitelist: &[WhitelistRule],
    ) -> HidePlan {
        self.prune_stale();
        let known: HashSet<i64> = self.hidden.iter().map(|t| t.hwnd).collect();

        let mut fresh: Vec<Target> = Vec::new();
        for t in targets {
            if known.contains(&t.hwnd)
                || fresh.iter().any(|f| f.hwnd == t.hwnd)
                || !self.wm.is_window(t.hwnd)
                || !self.wm.is_visible(t.hwnd)
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
            // 任务栏（Shell_TrayWnd）与桌面（Progman）带 WS_EX_TOOLWINDOW，不在枚举
            // 结果里，走 hide_current 时只有句柄。白名单要认得出它们就得先补上路径。
            if t.process_path.is_empty() && !whitelist.is_empty() {
                t.process_path = self.wm.process_path(t.pid);
            }
            if is_ignored(
                whitelist,
                &t.process_path,
                t.process_name(),
                IgnoreMode::Hide,
            ) {
                continue;
            }
            fresh.push(t);
        }

        let mut mute: Vec<ProcRecord> = Vec::new();
        if setting.mute_after_hide {
            for t in &fresh {
                if !self.muted.iter().any(|r| r.pid == t.pid)
                    && !mute.iter().any(|r| r.pid == t.pid)
                    && !is_ignored(
                        whitelist,
                        &t.process_path,
                        t.process_name(),
                        IgnoreMode::Mute,
                    )
                {
                    mute.push(self.proc_record(t.pid));
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

        HidePlan {
            send_pause: setting.send_before_hide && !fresh.is_empty(),
            // 解冻方式必须与冻结时一致。
            enhanced: if self.frozen.is_empty() {
                setting.enhanced_freeze
            } else {
                self.used_enhanced
            },
            fresh,
            mute,
            freeze,
        }
    }

    /// 执行计划：同步隐藏窗口（`SW_HIDE`），静音 / 冻结 / 暂停键经 [`Effects`]
    /// 施加。生产实现为异步队列，入队顺序即执行顺序。
    pub fn commit_hide(&mut self, plan: HidePlan) {
        // 暂停键排在最前：冻结后的进程收不到按键。
        if plan.send_pause {
            self.effects.send_pause();
        }

        for t in &plan.fresh {
            self.wm.hide(t.hwnd);
        }

        for r in &plan.mute {
            self.effects.mute(r.pid, true);
            self.muted.push(*r);
        }
        self.muted.sort_unstable_by_key(|r| r.pid);

        self.used_enhanced = plan.enhanced;
        // 冻结前必须等屏幕画完，否则被冻结的窗口会留下残影。整批只等一次。
        if !plan.freeze.is_empty() {
            self.effects.settle_before_freeze();
        }
        for r in &plan.freeze {
            self.effects.suspend(r.pid, plan.enhanced);
            self.frozen.push(*r);
        }
        self.frozen.sort_unstable_by_key(|r| r.pid);

        self.hidden.extend(plan.fresh);
    }

    /// plan + commit 的便捷封装，**不带白名单**；供测试与不需要白名单的调用方使用。
    /// 生产路径走 [`Self::plan_hide`]，白名单由调用方传入。
    pub fn apply_hide(&mut self, setting: &Setting, targets: &[Target], freeze_pids: &[u32]) {
        let plan = self.plan_hide(setting, targets, freeze_pids, &[]);
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

    /// 进程记录是否仍指向当初那个进程（PID 会被系统回收复用，须比对创建时刻）。
    fn proc_alive(&self, r: &ProcRecord) -> bool {
        let now = self.wm.process_start_time(r.pid);
        if now == 0 {
            return false;
        }
        r.created_at == 0 || now == r.created_at
    }

    /// 恢复全部隐藏窗口并撤销副作用；失效记录一律跳过并计入返回值。
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
            // 句柄仍存活且仍属于当初的进程才恢复，避免弹出复用同一句柄的无关窗口。
            if self.wm.is_window(t.hwnd) && (t.pid == 0 || self.wm.window_pid(t.hwnd) == t.pid) {
                self.wm.show(t.hwnd);
                outcome.shown += 1;
            } else {
                outcome.stale += 1;
                stale.push(t);
            }
        }
        outcome.refound = self.refind_stale(&hidden, &stale);

        let muted = std::mem::take(&mut self.muted);
        for r in &muted {
            if self.proc_alive(r) {
                self.effects.mute(r.pid, false);
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

        // 先解冻，否则窗口显示出来仍是卡死的。
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
            if self.wm.is_window(t.hwnd) {
                self.wm.show(t.hwnd);
            }
        }
        self.hidden = keep;

        let (unmute, keep): (Vec<ProcRecord>, Vec<ProcRecord>) = self
            .muted
            .iter()
            .copied()
            .partition(|r| pids.contains(&r.pid));
        for r in &unmute {
            if self.proc_alive(r) {
                self.effects.mute(r.pid, false);
            }
        }
        self.muted = keep;

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

    /// 当前隐藏状态的快照，用于崩溃恢复落盘。版本与开机时刻由 `recovery::save` 盖章。
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            hidden: self.hidden.clone(),
            frozen: self.frozen.clone(),
            muted: self.muted.clone(),
            enhanced: self.used_enhanced,
            ..Default::default()
        }
    }

    /// 在当前状态上叠加执行计划后的快照，供意图先行落盘。
    pub fn planned_snapshot(&self, plan: &HidePlan) -> Snapshot {
        let mut snapshot = self.snapshot();
        snapshot.hidden.extend(plan.fresh.iter().cloned());
        snapshot.frozen.extend(plan.freeze.iter().copied());
        snapshot.muted.extend(plan.mute.iter().copied());
        snapshot.enhanced = plan.enhanced;
        snapshot
    }

    /// 从崩溃前的快照恢复。快照整体有效性由调用方先行校验，
    /// 逐条身份校验由 [`Self::show`] 完成。
    pub fn restore_from(&mut self, snapshot: Snapshot) -> ShowOutcome {
        self.hidden = snapshot.hidden;
        self.frozen = snapshot.frozen;
        self.muted = snapshot.muted;
        self.used_enhanced = snapshot.enhanced;
        self.show()
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

    /// 复刻 agent 的隐藏编排：解析目标（含追溯回填）后交给控制器应用副作用。
    fn do_hide<W: WindowManager, E: Effects>(
        controller: &mut HideController<W, E>,
        config: &mut Config,
    ) {
        let windows = controller.enumerate();
        let foreground = controller.foreground();
        let (targets, _) = resolve_targets(config, &windows, foreground);
        let freezable = freezable_pids(&targets, &windows);
        let plan = controller.plan_hide(&config.setting, &targets, &freezable, config.whitelist());
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
            freezable_pids(&targets, &windows).is_empty(),
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
        assert_eq!(freezable_pids(&targets, &windows), vec![500]);
    }

    #[test]
    fn freeze_ignores_already_invisible_windows() {
        let windows = vec![
            win_pid("微信", 10, "WeChat.exe", 500, "C:\\WeChat.exe"),
            win_pid("后台窗口", 11, "WeChat.exe", 500, "C:\\WeChat.exe").with_visibility(false),
        ];
        let targets = vec![Target::bare(10, 500)];
        assert_eq!(freezable_pids(&targets, &windows), vec![500]);
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

    struct MockWm {
        windows: Vec<WindowInfo>,
        foreground: i64,
        visible: RefCell<HashSet<i64>>,
        /// 仍然存在的句柄；与 visible 是两回事。
        exists: RefCell<HashSet<i64>>,
        /// 覆写某句柄当前所属的 PID（模拟句柄被别的窗口复用）。
        pid_overrides: RefCell<HashMap<i64, u32>>,
        /// 覆写某进程的创建时刻（模拟 PID 被回收复用；0 = 进程已退出）。
        start_overrides: RefCell<HashMap<u32, i64>>,
        /// 枚举不到的进程的映像路径（如任务栏 / 桌面所属的 explorer.exe）。
        paths: RefCell<HashMap<u32, String>>,
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
            }
        }

        /// 登记一个枚举不到、但按 PID 能查出身份的窗口（任务栏 / 桌面即属此类）。
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
    }

    impl WindowManager for MockWm {
        // 与真实平台一致：不可见窗口也在枚举结果里，visible 标记如实反映当前状态。
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
        // 真实实现按 PID 查映像路径，因此枚举不到的窗口也能补出身份。
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
        pauses: RefCell<u32>,
        settles: RefCell<u32>,
    }

    impl Effects for MockEffects {
        fn mute(&self, pid: u32, mute: bool) {
            self.mutes.borrow_mut().push((pid, mute));
        }
        fn settle_before_freeze(&self) {
            *self.settles.borrow_mut() += 1;
        }
        fn suspend(&self, pid: u32, _enhanced: bool) {
            self.suspends.borrow_mut().push(pid);
        }
        fn resume(&self, pid: u32, _enhanced: bool) {
            self.resumes.borrow_mut().push(pid);
        }
        fn send_pause(&self) {
            *self.pauses.borrow_mut() += 1;
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
                refound: 0
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
            vec![ProcRecord {
                pid: 10,
                created_at: start_of(10)
            }]
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
        let freezable = freezable_pids(&targets, &windows);

        let plan = controller.plan_hide(&config.setting, &targets, &freezable, &[]);
        let planned = controller.planned_snapshot(&plan);
        controller.commit_hide(plan);
        let actual = controller.snapshot();

        // 落盘发生在动作前，故意图快照须与提交后的状态等价。
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
            muted: vec![ProcRecord {
                pid: 10,
                created_at: start_of(10),
            }],
            enhanced: false,
            ..Default::default()
        });

        assert_eq!(
            outcome,
            ShowOutcome {
                shown: 1,
                stale: 0,
                refound: 0
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
                refound: 0
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
                refound: 1
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

    #[test]
    fn already_invisible_windows_are_not_recorded_or_restored() {
        let setting = Setting {
            hide_current: false,
            mute_after_hide: true,
            ..Setting::default()
        };
        // 10 存在但已不可见，20 可见；两者都被规则命中。
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
        assert_eq!(controller.hidden_count(), 1, "已不可见的窗口不应入集");
        assert_eq!(
            *controller.effects.mutes.borrow(),
            vec![(600, true)],
            "只对真正被隐藏的窗口施加副作用"
        );

        let outcome = controller.show();
        assert_eq!(
            outcome,
            ShowOutcome {
                shown: 1,
                stale: 0,
                refound: 0
            }
        );
        assert!(controller.wm.is_visible(20), "记事本应恢复");
        assert!(
            !controller.wm.is_visible(10),
            "Steam 自己藏起来的窗口不得被恢复弹出"
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

    /// 忽略隐藏：命中的窗口连隐藏目标都进不去，规则照写也不生效。
    #[test]
    fn ignored_windows_never_become_hide_targets() {
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
        assert_eq!(ids(&targets), vec![(20, 20)], "白名单命中的窗口应被剔除");
        assert_eq!(
            outcomes,
            vec![RuleOutcome::Live, RuleOutcome::Live],
            "规则本身仍然解析成功，只是目标被白名单拦下"
        );
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
        assert!(targets.is_empty());
    }

    /// 回归：任务栏（`Shell_TrayWnd`）与桌面（`Progman`）带 `WS_EX_TOOLWINDOW`，
    /// **不在枚举结果里**，走「同时隐藏当前活动窗口」时只有一个句柄。
    /// `resolve_targets` 那时无从判定身份，必须由 `plan_hide` 补查路径后再过白名单，
    /// 否则白名单了 explorer.exe 也照样把桌面和任务栏藏掉。
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

        let plan = controller.plan_hide(&config.setting, &targets, &[], config.whitelist());
        assert!(plan.fresh.is_empty(), "补查路径后应被白名单挡下");
        controller.commit_hide(plan);
        assert!(controller.wm.is_visible(TASKBAR), "任务栏必须还在");
        assert!(!controller.is_hidden());
    }

    /// 上一条的对照：没被白名单的枚举外窗口仍然照常隐藏。
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
        let plan = controller.plan_hide(&config.setting, &targets, &[], config.whitelist());
        assert_eq!(plan.fresh.len(), 1);
        controller.commit_hide(plan);
        assert!(!controller.wm.is_visible(TASKBAR));
    }

    /// 恢复工具的手动隐藏传空白名单：勾了哪个窗口就藏哪个，软偏好不该拦。
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

        let plan = controller.plan_hide(&setting, &[Target::bare(TASKBAR, 0)], &[], &[]);
        assert_eq!(plan.fresh.len(), 1, "空白名单不拦任何窗口");
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

    /// 冻结白名单按映像名剔除；查不到名字的 PID 保留（拿不到身份不擅自放行）。
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

    /// 展开子进程树后，explorer 树里的 ZoneDeck 自己必须被内置保护挡下 —— 冻住它
    /// 就再也解不开了。白名单为空时同样生效。
    #[test]
    fn freeze_whitelist_protects_zonedeck_inside_an_expanded_tree() {
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
        assert_eq!(got, vec![100, 400], "核心与配置程序必须留下");
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
}
