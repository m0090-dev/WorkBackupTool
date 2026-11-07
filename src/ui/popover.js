/**
 * ポップオーバー(モード設定・作業対象メニュー・世代メニュー)の開閉。
 * index.html 側で、ボタンに data-popover="<ポップオーバーのid>" を付ける。
 *   data-placement="above" ... ボタンの上に開く(既定は下)
 * ポップオーバー内の要素に data-close を付けると、クリック時に閉じる。
 */
export function closePopovers() {
  document
    .querySelectorAll(".popover")
    .forEach((p) => p.classList.add("hidden"));
}

function openPopover(trigger) {
  const pop = document.getElementById(trigger.dataset.popover);
  if (!pop) return;
  const wasOpen = !pop.classList.contains("hidden");
  closePopovers();
  if (wasOpen) return;

  pop.classList.remove("hidden");
  const r = trigger.getBoundingClientRect();
  const w = pop.offsetWidth;
  const h = pop.offsetHeight;
  const margin = 8;

  let left = r.left;
  if (left + w > window.innerWidth - margin) left = r.right - w;
  left = Math.max(margin, Math.min(left, window.innerWidth - w - margin));

  let top =
    trigger.dataset.placement === "above" ? r.top - h - 4 : r.bottom + 4;
  top = Math.max(margin, Math.min(top, window.innerHeight - h - margin));

  pop.style.left = `${left}px`;
  pop.style.top = `${top}px`;
}

export function initPopovers() {
  document.addEventListener("click", (e) => {
    const trigger = e.target.closest("[data-popover]");
    if (trigger) {
      openPopover(trigger);
      return;
    }
    if (e.target.closest(".popover")) {
      if (e.target.closest("[data-close]")) closePopovers();
      return;
    }
    closePopovers();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closePopovers();
  });
  window.addEventListener("blur", closePopovers);
}
