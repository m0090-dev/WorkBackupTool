import os
import urllib.request
import zipfile
import shutil
from pathlib import Path

def setup_hdiff():
    # v4.12.0 などのタグ名
    tag_name = os.getenv("HDIFF_VERSION", "v4.12.0")
    # GitHub Actions の matrix から取得 (windows-latest, macos-latest, etc.)
    gh_platform = os.getenv("PLATFORM", "windows-latest")
    # Tauri 用のターゲットトリプル (x86_64-pc-windows-msvc, aarch64-apple-darwin, etc.)
    target_triple = os.getenv("TARGET", "")

    # HDiffPatch 独自の OS 命名規則にマッピング
    if "windows" in gh_platform:
        os_name = "windows64"
    elif "macos" in gh_platform:
        os_name = "macos"
    else:
        os_name = "linux64"

    file_name = f"hdiffpatch_{tag_name}_bin_{os_name}.zip"
    url = f"https://github.com/sisong/HDiffPatch/releases/download/{tag_name}/{file_name}"
    
    zip_path = "hdiff.zip"
    extract_path = "temp_hdiff"
    bin_dir = Path("src-tauri/binaries")

    os.makedirs(bin_dir, exist_ok=True)

    print(f"Target URL: {url}")
    if target_triple:
        print(f"Target Triple: {target_triple}")

    try:
        # 1. ダウンロード
        print(f"Downloading {file_name}...")
        opener = urllib.request.build_opener()
        opener.addheaders = [('User-Agent', 'Mozilla/5.0')]
        urllib.request.install_opener(opener)
        urllib.request.urlretrieve(url, zip_path)

        # 2. 解凍
        with zipfile.ZipFile(zip_path, 'r') as zip_ref:
            zip_ref.extractall(extract_path)

        # 3. バイナリの抽出・リネーム・配置
        found_count = 0
        for root, _, files in os.walk(extract_path):
            for file in files:
                if file.startswith("hdiffz") or file.startswith("hpatchz"):
                    file_path = Path(root) / file
                    
                    # 拡張子 (.exe 等) とファイル名本体を分ける
                    stem = file_path.stem
                    suffix = file_path.suffix

                    # Tauri 用のリネームルールを適用
                    # 例: hdiffz -> hdiffz-x86_64-pc-windows-msvc.exe
                    if target_triple:
                        new_name = f"{stem}-{target_triple}{suffix}"
                    else:
                        new_name = f"{stem}{suffix}"

                    dest = bin_dir / new_name
                    shutil.copy2(file_path, dest)
                    
                    print(f"Placed: {new_name}")
                    if "windows" not in gh_platform:
                        os.chmod(dest, 0o755)
                    found_count += 1
        
        if found_count == 0:
            raise Exception("ZIPの中にバイナリが見つかりませんでした。")

    except Exception as e:
        print(f"Error: {e}")
        exit(1)
    finally:
        if os.path.exists(zip_path): os.remove(zip_path)
        if os.path.exists(extract_path): shutil.rmtree(extract_path)

if __name__ == "__main__":
    setup_hdiff()




