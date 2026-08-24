//! 隐藏 / 恢复结果与热键失败的日志文案格式化。

use windows::Win32::Foundation::ERROR_HOTKEY_ALREADY_REGISTERED;
use zonedeck_common::Config;

use crate::hide::{HidePlan, RuleOutcome, ShowOutcome};
use crate::logging;

use super::Trigger;

/// 日志中指代一条窗口规则的写法：序号 + 进程名，不含标题。
pub(super) fn rule_label(index: usize, rule: &zonedeck_common::WindowRule) -> String {
    let kind = if rule.is_regex() { "正则" } else { "精确" };
    let process = if rule.process.is_empty() {
        "未知进程"
    } else {
        &rule.process
    };
    format!("{kind}窗口规则 #{}（{process}）", index + 1)
}

/// 摘要式记录本次隐藏：明细记 debug，规则未匹配到窗口记 warn。
pub(super) fn log_hide(
    trigger: Trigger,
    config: &Config,
    outcomes: &[RuleOutcome],
    plan: &HidePlan,
) {
    for (index, (rule, outcome)) in config.window_rules.iter().zip(outcomes).enumerate() {
        match outcome {
            RuleOutcome::Reacquired => logging::debug(&format!(
                "{} 的句柄已失效（目标程序重启过），已重新匹配并更新规则",
                rule_label(index, rule)
            )),
            RuleOutcome::Missing => logging::warn(&format!(
                "{} 未匹配到任何窗口（可能已关闭或标题已变），本次不隐藏它",
                rule_label(index, rule)
            )),
            _ => {}
        }
    }
    if plan.fresh.is_empty() {
        logging::debug(&format!("{trigger}触发隐藏：没有新的目标窗口"));
        return;
    }
    logging::debug(&format!(
        "{trigger}触发隐藏 {} 个窗口: {}",
        plan.fresh.len(),
        summarize(plan.fresh.iter().map(|t| t.describe()))
    ));
    let untouched = plan
        .fresh
        .iter()
        .filter(|t| t.restore == crate::platform::Restore::Skip)
        .count();
    if untouched > 0 {
        logging::debug(&format!(
            "其中 {untouched} 个窗口隐藏前就不可见，本次不改动它们的显示状态，恢复时也不会弹出"
        ));
    }
    if !plan.freeze.is_empty() {
        logging::debug(&format!(
            "冻结 {} 个进程（增强={}，清空工作集={}）: {}",
            plan.freeze.len(),
            plan.enhanced,
            plan.trim,
            summarize(plan.freeze.iter().map(|r| r.pid.to_string()))
        ));
    }
}

/// 记录恢复结果：有记录未能找回时记 warn，否则记 debug。
pub(super) fn log_show(trigger: Trigger, outcome: ShowOutcome) {
    let skipped = if outcome.skipped > 0 {
        format!("；{} 条记录隐藏前就不可见，未予显示", outcome.skipped)
    } else {
        String::new()
    };
    let lost = outcome.stale.saturating_sub(outcome.refound);
    if lost > 0 {
        logging::warn(&format!(
            "{trigger}触发恢复：显示 {} 个窗口；{} 条记录的句柄已失效，其中 {} 个已按进程与标题找回，{lost} 个未能找回{skipped}",
            outcome.shown, outcome.stale, outcome.refound
        ));
    } else {
        logging::debug(&format!(
            "{trigger}触发恢复显示 {} 个窗口{skipped}",
            outcome.shown
        ));
    }
}

/// 拼接清单，最多列出前 8 项，其余以「等 N 项」收尾。
fn summarize(items: impl Iterator<Item = String>) -> String {
    const MAX: usize = 8;
    let all: Vec<String> = items.collect();
    if all.len() <= MAX {
        all.join("、")
    } else {
        format!("{}、等 {} 项", all[..MAX].join("、"), all.len())
    }
}

/// 热键注册失败的日志文案；只在错误码为 1409 时断言被占用。
pub(super) fn hotkey_failure_message(label: &str, raw: &str, e: &windows::core::Error) -> String {
    if e.code() == ERROR_HOTKEY_ALREADY_REGISTERED.to_hresult() {
        format!("{label}热键注册失败，已被其他程序占用，该热键不生效: {raw}")
    } else {
        format!(
            "{label}热键注册失败，该热键不生效: {raw} — {}",
            crate::util::win_err(e)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_label_names_the_process_and_never_the_window_title() {
        let mut rule = zonedeck_common::WindowRule::from_window(&zonedeck_common::WindowInfo::new(
            "与某人的聊天",
            10,
            "WeChat.exe",
            2001,
            "C:\\WeChat.exe",
        ));
        let label = rule_label(0, &rule);
        assert_eq!(label, "精确窗口规则 #1（WeChat.exe）");
        assert!(!label.contains("与某人的聊天"), "标题属隐私，不得进日志");

        rule.regex = Some("机密.*".to_string());
        let label = rule_label(2, &rule);
        assert_eq!(label, "正则窗口规则 #3（WeChat.exe）");
        assert!(!label.contains("机密"), "正则本体也可能含标题片段，不写出");
    }
}
