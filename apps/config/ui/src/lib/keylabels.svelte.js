// 扩展主键（小键盘 / OEM 符号键）在当前键盘布局下的显示字符。
// 配置里存的一律是位置名（OEM_1、NumpadAdd…），只有界面显示走这张表。

import { invoke } from "./ipc.js";

const labels = $state({ map: {} });

invoke("key_labels")
  .then((m) => (labels.map = m || {}))
  .catch(() => {});

/** 位置名 → 当前布局下的字符；没有映射时原样返回。 */
export function keyLabel(name) {
  return labels.map[name] || name;
}
