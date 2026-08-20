// 自动保存调度：debounce 合并连续改动，写盘互不并发，关窗前可 flush。

/**
 * @param {() => Promise<boolean>} write 实际写盘；返回是否成功。
 * @param {number} delayMs debounce 间隔。
 */
export function createAutosave(write, delayMs = 600) {
  let timer = null;
  /** 有改动排队待存。 */
  let pending = false;
  /** 正在写盘的那一次；flush 与后续写入都须等它完成。 */
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
    // pending 为 false 即改动已被某次已启动的写捕获，遗留定时器作废。
    if (!pending) return;
    // 上一笔还在写盘，等它完成后立即补写。
    if (inFlight) inFlight.then(() => pending && run());
    else run();
  }

  return {
    /** 是否还有未落盘的改动。 */
    get dirty() {
      return pending || inFlight !== null;
    },

    /** 安排一次自动保存；连续改动只在停顿后写一次盘。 */
    schedule(delay = delayMs) {
      pending = true;
      clearTimeout(timer);
      timer = setTimeout(onTimer, delay);
    },

    /** 立即写盘全部未落盘的改动；返回是否成功，无待存改动视为成功。 */
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
