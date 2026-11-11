#!/usr/bin/env python3
"""
WorkBackupTool 世代管理の動作確認ツール(対話形式)

やること:
  - テスト用ファイルを指定サイズで作る
  - ファイルを意図的に変更する(追記 / 部分書き換え / 途中に挿入)
  - 「変更 → 待つ → 変更 …」を自動で繰り返す(待っている間にアプリで実行する)
  - バックアップフォルダを読んで、世代ごとのベース/差分/累計/比率を表示する

使い方:
  python scripts/gen_test.py
  python scripts/gen_test.py --file path/to/target.bin --threshold 1.0

外部ライブラリは不要(標準ライブラリのみ)。
※ 既定ではテスト用ファイル scripts/_gen_test_work/sample.bin を使う。
  実データのファイルを指定した場合は、書き換える前に確認する。
"""
import argparse
import os
import random
import re
import sys
import tempfile
import time
from pathlib import Path

WORK_DIR = Path(__file__).resolve().parent / "_gen_test_work"
DEFAULT_TARGET = WORK_DIR / "sample.bin"
GATE = 100 * 1024  # アプリ側: ベースが100KB以下なら世代交代しない
CHUNK = 1024 * 1024

UNITS = {"": 1, "b": 1, "k": 1024, "kb": 1024, "m": 1024**2, "mb": 1024**2, "g": 1024**3, "gb": 1024**3}


# ---------------------------------------------------------------- 入出力ヘルパー
def parse_size(text):
    m = re.fullmatch(r"\s*([0-9]+(?:\.[0-9]+)?)\s*([a-zA-Z]*)\s*", text)
    if not m or m.group(2).lower() not in UNITS:
        raise ValueError(f"サイズを読めません: {text!r}(例: 500, 200KB, 1.5MB)")
    return int(float(m.group(1)) * UNITS[m.group(2).lower()])


def fmt_size(n):
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024 or unit == "GB":
            return f"{n:.0f} {unit}" if unit == "B" else f"{n:.2f} {unit}"
        n /= 1024


def ask(prompt, default=None):
    suffix = f" [{default}]" if default is not None else ""
    try:
        s = input(f"{prompt}{suffix}: ").strip()
    except EOFError:
        raise SystemExit(0)
    return s if s else (str(default) if default is not None else "")


def ask_size(prompt, default):
    while True:
        try:
            return parse_size(ask(prompt, default))
        except ValueError as e:
            print("  ", e)


def ask_int(prompt, default):
    while True:
        try:
            return int(ask(prompt, default))
        except ValueError:
            print("   整数で入力してください")


def ask_yes(prompt, default=True):
    d = "Y/n" if default else "y/N"
    s = ask(f"{prompt} ({d})", "").lower()
    return default if not s else s.startswith("y")


class State:
    def __init__(self, target, threshold, seed):
        self.target = target
        self.threshold = threshold
        self.rng = random.Random(seed)

    def rand_bytes(self, n):
        if hasattr(self.rng, "randbytes"):
            return self.rng.randbytes(n)
        return os.urandom(n)


# ---------------------------------------------------------------- ファイル作成・変更
def write_random(st, path, size):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "wb") as f:
        left = size
        while left > 0:
            n = min(CHUNK, left)
            f.write(st.rand_bytes(n))
            left -= n


def m_append(st, size):
    with open(st.target, "ab") as f:
        left = size
        while left > 0:
            n = min(CHUNK, left)
            f.write(st.rand_bytes(n))
            left -= n


def m_scatter(st, total, patches):
    """サイズは変えずに、複数の場所を total バイト分だけ書き換える(上書き編集の再現)"""
    fsize = st.target.stat().st_size
    if fsize == 0:
        raise ValueError("ファイルが空です")
    patches = max(1, patches)
    each = max(1, total // patches)
    with open(st.target, "r+b") as f:
        for _ in range(patches):
            n = min(each, fsize)
            pos = st.rng.randint(0, fsize - n)
            f.seek(pos)
            f.write(st.rand_bytes(n))


def m_insert(st, size):
    """ランダムな位置に挿入(以降の内容がずれる。ファイル全体を書き直す)"""
    fsize = st.target.stat().st_size
    pos = st.rng.randint(0, fsize)
    tmp = tempfile.NamedTemporaryFile(delete=False, dir=st.target.parent, suffix=".tmp")
    try:
        with open(st.target, "rb") as src, tmp:
            left = pos
            while left > 0:
                buf = src.read(min(CHUNK, left))
                if not buf:
                    break
                tmp.write(buf)
                left -= len(buf)
            tmp.write(st.rand_bytes(size))
            while True:
                buf = src.read(CHUNK)
                if not buf:
                    break
                tmp.write(buf)
        os.replace(tmp.name, st.target)
    finally:
        if os.path.exists(tmp.name):
            os.remove(tmp.name)


MUTATIONS = {
    "1": ("追記 (append)", "追記するサイズ", "10KB"),
    "2": ("部分書き換え (scatter)", "書き換える合計サイズ", "10KB"),
    "3": ("途中に挿入 (insert)", "挿入するサイズ", "10KB"),
}


def run_mutation(st, kind, size):
    if kind == "1":
        m_append(st, size)
    elif kind == "2":
        m_scatter(st, size, patches=8)
    elif kind == "3":
        m_insert(st, size)


# ---------------------------------------------------------------- バックアップ状況の表示
def default_backup_dir(target):
    stem = target.name if target.is_dir() else target.stem
    return target.parent / f"wbt_backup_{stem}"


def path_size(p):
    if p.is_file():
        return p.stat().st_size
    total = 0
    for root, _, files in os.walk(p):
        for name in files:
            try:
                total += (Path(root) / name).stat().st_size
            except OSError:
                pass
    return total


def inspect(st, backup_dir=None):
    root = Path(backup_dir) if backup_dir else default_backup_dir(st.target)
    print(f"\n  バックアップ先: {root}")
    if not root.exists():
        print("  (まだありません。アプリで1回バックアップを実行してください)")
        return
    gens = []
    for d in root.iterdir():
        m = re.match(r"^base(\d+)_", d.name)
        if d.is_dir() and m:
            gens.append((int(m.group(1)), d.name, d))
    gens.sort()
    if not gens:
        print("  (世代フォルダがありません)")
        return

    name = st.target.name
    print(f"  閾値: {st.threshold}  (累計 ÷ ベース がこれを超えると次の世代)\n")
    print(f"  {'世代':<6}{'ベース':>12}{'差分数':>7}{'差分累計':>12}{'比率':>8}  状態")
    for i, (idx, dname, d) in enumerate(gens):
        base = d / f"{name}.base"
        base_size = path_size(base) if base.exists() else 0
        diffs = [p for p in d.iterdir()
                 if p.is_file() and p.name.startswith(name + ".") and p.name.endswith(".diff")]
        total = sum(p.stat().st_size for p in diffs)
        ratio = (total / base_size) if base_size else 0.0
        latest = i == len(gens) - 1
        if base_size <= GATE:
            note = "ベース100KB以下: 交代しない"
        elif ratio > st.threshold:
            note = "閾値超え(次のバックアップで交代)" if latest else "閾値超え"
        else:
            left = int(base_size * st.threshold - total)
            note = f"あと約 {fmt_size(max(left, 0))} で交代" if latest else "OK"
        mark = " ★" if latest else ""
        print(f"  {idx:<6}{fmt_size(base_size):>12}{len(diffs):>7}{fmt_size(total):>12}{ratio:>8.2f}  {note}{mark}")
    print()


# ---------------------------------------------------------------- メニュー
def set_target(st):
    path = Path(ask("対象ファイルのパス", st.target)).expanduser().resolve()
    if path.is_dir():
        print("  フォルダは未対応です。ファイルを指定してください。")
        return
    if not path.exists():
        size = ask_size("存在しません。作成します。サイズ", "500KB")
        write_random(st, path, size)
        print(f"  作成しました: {path} ({fmt_size(size)})")
    else:
        in_work = WORK_DIR in path.parents
        print(f"  既存のファイルです({fmt_size(path.stat().st_size)})")
        if not in_work and not ask_yes("  ★ このファイルを直接書き換えます。続けますか?", False):
            return
    st.target = path


def need_target(st):
    if st.target.exists():
        return True
    print("  対象ファイルがありません。先に 1 で設定してください。")
    return False


def menu_mutate(st, kind):
    if not need_target(st):
        return
    label, prompt, default = MUTATIONS[kind]
    size = ask_size(prompt, default)
    run_mutation(st, kind, size)
    print(f"  {label}: 完了 → 現在 {fmt_size(st.target.stat().st_size)}")


def menu_auto(st):
    if not need_target(st):
        return
    print("  変更の種類:")
    for k, (label, _, _) in MUTATIONS.items():
        print(f"    {k}) {label}")
    kind = ask("選択", "1")
    if kind not in MUTATIONS:
        print("  無効な選択です")
        return
    size = ask_size("1ラウンドあたりのサイズ", MUTATIONS[kind][2])
    rounds = ask_int("ラウンド数", 5)
    pause = ask("待ち方: Enter=手動(アプリで実行したらEnter) / 数字=秒数で自動", "Enter")
    show = ask_yes("毎ラウンド後にバックアップ状況を表示する", True)

    for i in range(1, rounds + 1):
        run_mutation(st, kind, size)
        print(f"\n[{i}/{rounds}] {MUTATIONS[kind][0]} {fmt_size(size)} → 現在 {fmt_size(st.target.stat().st_size)}")
        if pause.lower() in ("enter", ""):
            if ask("  アプリでバックアップを実行して Enter(q で中断)", "").lower() == "q":
                print("  中断しました")
                break
        else:
            secs = float(pause)
            print(f"  {secs:g} 秒待ちます(この間にアプリで実行)…")
            time.sleep(secs)
        if show:
            inspect(st)


def main():
    ap = argparse.ArgumentParser(description="WorkBackupTool 世代管理の動作確認ツール")
    ap.add_argument("--file", help="対象ファイル(省略時は scripts/_gen_test_work/sample.bin)")
    ap.add_argument("--threshold", type=float, default=1.0, help="世代交代の閾値(既定 1.0)")
    ap.add_argument("--seed", type=int, default=None, help="乱数シード(再現したいとき)")
    args = ap.parse_args()

    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")

    target = Path(args.file).expanduser().resolve() if args.file else DEFAULT_TARGET
    st = State(target, args.threshold, args.seed)

    while True:
        exists = fmt_size(st.target.stat().st_size) if st.target.exists() else "未作成"
        print("\n=== 世代管理テスト ===")
        print(f"  対象: {st.target}  ({exists})")
        print(f"  閾値: {st.threshold}")
        print("  1) 対象ファイルを設定 / 作成")
        print("  2) 追記 (append)")
        print("  3) 部分書き換え (scatter)")
        print("  4) 途中に挿入 (insert)")
        print("  5) 自動ラウンド(変更 → 待つ → 状況表示 を繰り返す)")
        print("  6) バックアップ状況を表示")
        print("  7) 閾値を変更")
        print("  q) 終了")
        choice = ask("選択", "").lower()
        try:
            if choice == "1":
                set_target(st)
            elif choice == "2":
                menu_mutate(st, "1")
            elif choice == "3":
                menu_mutate(st, "2")
            elif choice == "4":
                menu_mutate(st, "3")
            elif choice == "5":
                menu_auto(st)
            elif choice == "6":
                inspect(st, ask("バックアップ先(Enterで既定)", "") or None)
            elif choice == "7":
                st.threshold = float(ask("閾値", st.threshold))
            elif choice in ("q", "quit", "exit"):
                break
        except (ValueError, OSError) as e:
            print(f"  エラー: {e}")
        except KeyboardInterrupt:
            print("\n  中断しました")


if __name__ == "__main__":
    main()
