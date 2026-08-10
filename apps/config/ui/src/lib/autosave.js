// 自动保存调度：debounce 合并连续改动，写盘互不并发，关窗前可 flush。
// 与具体写盘方式解耦（write 由调用方注入），便于单独测试。

/**
 * @param {() => Promise<boolean>} write 实际写盘；返回是否成功。
 * @param {number} delayMs debounce 间隔。
 */
export function createAutosave(write, delayMs = 600) {
  let timer = null;
  /** 有改动排队待存（debounce 期间为 true）。 */
  let pending = false;
  /** 正在写盘的那一次；flush 与后续写入都须等它完成，不并发写。 */
  let inFlight = null;

  function run() {
    pending = false;
    inFlight = (async () => {
      try {
        return await write();
      } catch {
        return false;
      } finally {
        inFlight = null;
      }
    })();
    return inFlight;
  }

  function onTimer() {
    // pending 在 run() 的同步段清零：为 false 即改动已被某次已启动的写捕获，
    // 到点的遗留定时器直接作废，不产生冗余写盘。
    if (!pending) return;
    // 上一笔还在写盘：既不并发写也不丢改动，等它完成后立即补写。
    if (inFlight) inFlight.then(() => pending && run());
    else run();
  }

  return {
    /** 是否还有未落盘的改动（排队中或写盘中）。 */
    get dirty() {
      return pending || inFlight !== null;
    },

    /** 安排一次自动保存；连续改动只在停顿后写一次盘。 */
    schedule(delay = delayMs) {
      pending = true;
      clearTimeout(timer);
      timer = setTimeout(onTimer, delay);
    },

    /**
     * 立即写盘全部未落盘的改动；返回是否成功（无待存改动视为成功）。
     * 写盘途中排队的补写也在等待范围内，返回时必然已无未落盘改动。
     */
    async flush() {
      let ok = true;
      while (pending || inFlight) {
        clearTimeout(timer);
        ok = inFlight ? await inFlight : await run();
      }
      return ok;
    },
  };
}
