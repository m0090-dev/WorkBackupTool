/**
 * キー入力(KeyboardEvent)から、ショートカット文字列("Ctrl+Alt+B" など)を作る。
 * 設定画面のキー割り当て欄と、グローバルショートカットの登録で共通に使う。
 */
const MODIFIER_KEYS = ["Control", "Shift", "Alt", "Meta", "OS"];

const NAMED_KEYS = {
  Space: "Space",
  Enter: "Enter",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Insert: "Insert",
  Minus: "-",
  Equal: "=",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Backquote: "`",
};

// 物理キー(e.code)で判定するので、キーボード配列や IME の状態に左右されない
function keyName(e) {
  if (/^Key[A-Z]$/.test(e.code)) return e.code.slice(3);
  if (/^Digit[0-9]$/.test(e.code)) return e.code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(e.code)) return e.code;
  return NAMED_KEYS[e.code] ?? null;
}

/**
 * @returns {string|null} ショートカット文字列。修飾キーだけ・割り当てできないキー・
 *   修飾キー無しの通常キー(F1〜F24 以外)のときは null
 */
export function shortcutFromEvent(e) {
  if (MODIFIER_KEYS.includes(e.key)) return null;
  const key = keyName(e);
  if (!key) return null;

  const mods = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");

  const isFunctionKey = /^F\d+$/.test(key);
  if (mods.length === 0 && !isFunctionKey) return null;
  return [...mods, key].join("+");
}
