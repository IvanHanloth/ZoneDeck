// 正则「可能过宽」检查：保存前把配置里的全部正则送去后端试匹配随机样本。
// 判定本身在 Rust 侧，这里只负责收集正则、记住确认状态、产出要标红的集合。

/** 样本条数，与核心 `matching::BREADTH_SAMPLES` 一致。 */
export const BREADTH_SAMPLES = 200;

/** 配置里带正则的三个列表；`kind` 决定弹窗里显示的来源名。 */
const REGEX_LISTS = [
  { kind: "window", field: "window_rules" },
  { kind: "process", field: "process_rules" },
  { kind: "whitelist", field: "whitelist" },
];

/** 收集配置里全部非空正则，返回 `[{ kind, index, pattern }]`；重复项全部列出。 */
export function collectRegexPatterns(config) {
  const found = [];
  for (const { kind, field } of REGEX_LISTS) {
    const rules = config?.[field];
    if (!Array.isArray(rules)) continue;
    rules.forEach((rule, index) => {
      const pattern = rule?.regex;
      if (typeof pattern === "string" && pattern !== "") {
        found.push({ kind, index, pattern });
      }
    });
  }
  return found;
}

/**
 * 过宽检查器。两种确认分开记：确认无误的连红框一起撤掉，「我知道了」的只是不再
 * 打断保存。都只存内存，换一次会话重新提醒。
 *
 * @param {(patterns: string[]) => Promise<Array<number|null>>} measure
 *        送一批正则去后端，回来是各自的命中条数（null = 正则编译失败）。
 * @param {number} limit 命中超过它即判定过宽；与核心 `BREADTH_LIMIT` 一致。
 */
export function createBreadthGuard(measure, limit = BREADTH_SAMPLES / 2) {
  /** 点过「仍然保存」的正则：不再提醒，也不再标红。 */
  const acknowledged = new Set();
  /** 点过「我知道了」的正则：不再弹窗，但保持标红。 */
  const dismissed = new Set();

  return {
    /**
     * 检查一份配置，返回 `{ broad, toWarn }`：
     * - `broad`：判为过宽、且未被确认无误的条目，界面据此标红；
     * - `toWarn`：其中还没弹过窗的，为空表示这次不必打断保存。
     *
     * 后端出错时两者皆空。
     */
    async inspect(config) {
      const empty = { broad: [], toWarn: [] };
      const found = collectRegexPatterns(config);
      // 同一条正则只测一次。
      const unique = [...new Set(found.map((f) => f.pattern))];
      if (unique.length === 0) return empty;

      let counts;
      try {
        counts = await measure(unique);
      } catch {
        return empty;
      }
      if (!Array.isArray(counts)) return empty;

      const hits = new Map();
      unique.forEach((pattern, i) => {
        const n = counts[i];
        if (typeof n === "number" && n > limit) hits.set(pattern, n);
      });

      const seen = new Set();
      const broad = found
        .filter((f) => hits.has(f.pattern) && !acknowledged.has(f.pattern))
        .filter((f) => (seen.has(f.pattern) ? false : seen.add(f.pattern)))
        .map((f) => ({ kind: f.kind, pattern: f.pattern, hits: hits.get(f.pattern) }));

      return { broad, toWarn: broad.filter((i) => !dismissed.has(i.pattern)) };
    },

    /** 记下已确认无误的正则，此后不再提醒、不再标红。 */
    acknowledge(patterns) {
      for (const p of patterns) acknowledged.add(p);
    },

    /** 记下已知晓但暂不修改的正则：不再弹窗，红框保留。 */
    dismiss(patterns) {
      for (const p of patterns) dismissed.add(p);
    },
  };
}
