// 热键录制状态机：把 capture 送来的一串按键快照归约成待保存的组合。
//
// 快照形如 `{ modifiers, keys, down, unsupported }`，`keys` 是此刻按住的主键。
// 归约成纯函数是因为规则有不少边角：多主键要等按齐、纯修饰键要等全松开、
// 长按的重复按下不算新输入、定稿后不松手也得能接着录下一个组合。

import { joinCombo } from "./hotkey.js";

/** 组合里一共几个键（修饰键 + 主键）。 */
const size = (c) => (c.modifiers ? c.modifiers.split("+").length : 0) + c.keys.length;

const NOTHING = { modifiers: "", keys: [] };

/** 录制器初始状态；`draft` 是打开对话框时的现有组合。 */
export function initRecorder(draft = "") {
  return {
    /** 待保存的组合。 */
    draft,
    /** 此刻按住的键。 */
    live: NOTHING,
    /** 本轮按住过的最大组合。 */
    peak: NOTHING,
    /** 本轮已定稿；松手途中据此不再跟着 live 掉键。 */
    committed: false,
    /** 上一次按了热键表里没有的键。 */
    unsupported: false,
  };
}

function commit(state) {
  return {
    ...state,
    draft: joinCombo(state.peak.modifiers, state.peak.keys),
    committed: true,
    unsupported: false,
  };
}

/** 吃进一条录制快照，返回新的录制状态。纯函数。 */
export function stepRecorder(state, snapshot) {
  const keys = snapshot.keys ?? [];
  const now = { modifiers: snapshot.modifiers || "", keys };
  const next = { ...state, live: now };

  if (snapshot.unsupported) return { ...next, unsupported: true };

  if (size(now) === 0) {
    // 手全松开：纯修饰键组合在这一刻定稿，之后重新起一轮。
    const done = !state.committed && size(state.peak) ? commit(next) : next;
    return { ...done, committed: false, peak: NOTHING };
  }

  if (!snapshot.down) {
    // 松手途中：本轮第一次松手即定稿，让 Q+W 这类组合录得到完整的一组。
    return state.committed ? next : commit(next);
  }

  // 长按会持续送出按下事件，按住的键没变多就不是新输入。
  // 少了这道判断，一个仍按住的修饰键会自己另起一轮，把刚录好的组合覆盖掉。
  if (size(now) <= size(state.live)) return next;

  // 定稿后又按下新键，说明用户要重录；本轮内按住的键只增不减，故 peak 直接跟上。
  return {
    ...next,
    committed: false,
    peak: { modifiers: now.modifiers, keys: [...keys] },
    unsupported: keys.length ? false : state.unsupported,
  };
}
