import os
import sys
import shutil
import argparse
import subprocess
import json
import re

def get_version_from_wails():
    """wails.json から productVersion を取得する共通関数"""
    wails_path = 'wails.json'
    try:
        with open(wails_path, 'r', encoding='utf-8') as f:
            wails_data = json.load(f)
            version = wails_data.get("info", {}).get("productVersion")
            return version
    except Exception as e:
        print(f"Error reading {wails_path}: {e}")
        return None

def update_app_config_version(new_version):
    """AppConfig.json の i18n 内バージョン記述を更新する"""
    if not new_version:
        print("Error: バージョンが指定されていないため AppConfig.json の更新をスキップします。")
        return

    config_path = 'frontend/src/assets/AppConfig.json'
    try:
        if not os.path.exists(config_path):
            print(f"Warning: {config_path} が見つからないためスキップします。")
            return

        with open(config_path, 'r', encoding='utf-8') as f:
            config_data = json.load(f)

        # 置換パターン: "App Name 0.0.1(1)" のような形式を想定
        target_pattern = r"([a-zA-Z0-9_-]+\s)([0-9.]+)(\([0-9]+\))"
        replacement = rf"\g<1>{new_version}\g<3>"

        if "i18n" in config_data:
            for lang in config_data["i18n"]:
                about_text = config_data["i18n"][lang].get("aboutText", "")
                if about_text:
                    updated_text = re.sub(target_pattern, replacement, about_text)
                    config_data["i18n"][lang]["aboutText"] = updated_text

        with open(config_path, 'w', encoding='utf-8') as f:
            json.dump(config_data, f, indent=2, ensure_ascii=False)
        
        print(f"Success: AppConfig.json をバージョン {new_version} に同期しました。")

    except Exception as e:
        print(f"Error updating AppConfig.json: {e}")

def create_release_package():
    # 1. 引数の設定
    parser = argparse.ArgumentParser(description="cg-file-backup packaging script")
    parser.add_argument("--ver", help="Override version string (default: read from wails.json)")
    parser.add_argument("--zip", action="store_true", help="Create a zip archive of the package")
    parser.add_argument("--clean", action="store_true", help="Remove folders after processing")
    
    args = parser.parse_args()

    # 2. バージョンの決定 (引数優先 > wails.json)
    version = args.ver if args.ver else get_version_from_wails()
    
    if not version:
        print("Error: バージョンを特定できませんでした。wails.json を確認するか --ver を指定してください。")
        sys.exit(1)

    # AppConfig.json 側の書き換えを実行
    update_app_config_version(version)

    project_name = "cg-file-backup"
    dist_dir = f"{project_name}-{version}"

    # 3. 出力用フォルダの準備
    if os.path.exists(dist_dir):
        shutil.rmtree(dist_dir)
    os.makedirs(dist_dir)

    print(f"--- Packaging {project_name} version {version} ---")

    # 4. build/bin 内のファイルをコピー
    target_build_dir = "build/bin"
    if not os.path.exists(target_build_dir):
        target_build_dir = "build"

    artifacts = []
    if os.path.exists(target_build_dir):
        for item in os.listdir(target_build_dir):
            src_path = os.path.join(target_build_dir, item)
            if os.path.isfile(src_path):
                print(f"Copying build artifact: {item}")
                shutil.copy2(src_path, dist_dir)
                artifacts.append(item)
    else:
        print("Warning: Build directory not found.")

    # 5. 外部依存ディレクトリのコピー
    required_bins = ["hdiff-bin", "bzip2-bin"]
    for bin_dir in required_bins:
        if os.path.exists(bin_dir):
            print(f"Copying external dependency: {bin_dir}")
            shutil.copytree(bin_dir, os.path.join(dist_dir, bin_dir))

    # 5.5 サードパーティライセンスディレクトリ (licenses/) のコピー
    # go-licenses save ./cmd/... --save_path=licenses で生成されたものを想定
    third_party_licenses_dir = "licenses"
    if os.path.exists(third_party_licenses_dir):
        print(f"Copying third-party licenses directory: {third_party_licenses_dir}")
        shutil.copytree(third_party_licenses_dir, os.path.join(dist_dir, third_party_licenses_dir))
    else:
        print(f"Warning: {third_party_licenses_dir} directory not found. Skip copying.")

    # 6. LICENSE/CREDITS/READMEのコピー
    found_license = None
    for pattern in ["LICENSE", "LICENSE.txt", "LICENSE.md"]:
        if os.path.exists(pattern):
            shutil.copy2(pattern, dist_dir)
            found_license = pattern
            break
    
    for pattern in ["CREDITS", "CREDITS.txt", "CREDITS.md"]:
        if os.path.exists(pattern):
            shutil.copy2(pattern, dist_dir)
            break

    for pattern in ["README", "README.txt", "README.md"]:
        if os.path.exists(pattern):
            shutil.copy2(pattern, dist_dir)
            break






    # 7. Linux環境での .deb パッケージ作成
    deb_file_path = f"{dist_dir}.deb"
    if sys.platform.startswith('linux'):
        print(f"--- Creating .deb package for Linux ---")
        deb_root = f"{dist_dir}_deb"
        if os.path.exists(deb_root): shutil.rmtree(deb_root)
        
        try:
            bin_path = os.path.join(deb_root, "usr/bin")
            doc_path = os.path.join(deb_root, f"usr/share/doc/{project_name}")
            os.makedirs(bin_path, exist_ok=True)
            os.makedirs(doc_path, exist_ok=True)
            os.makedirs(os.path.join(deb_root, "DEBIAN"), exist_ok=True)

            for item in artifacts:
                if not (item.endswith('.exe') or item.endswith('.dll')):
                    target = shutil.copy2(os.path.join(dist_dir, item), bin_path)
                    os.chmod(target, 0o755)
                    print(f"Included in deb: {item}")
            
            for rb in required_bins:
                src_rb = os.path.join(dist_dir, rb)
                if os.path.exists(src_rb):
                    dest_rb = os.path.join(bin_path, rb)
                    os.makedirs(dest_rb, exist_ok=True)
                    for f in os.listdir(src_rb):
                        if not (f.endswith('.exe') or f.endswith('.dll')):
                            src_f = os.path.join(src_rb, f)
                            dest_f = shutil.copy2(src_f, dest_rb)
                            os.chmod(dest_f, 0o755)

            # .deb内にも licenses/ を含める (usr/share/doc 配下に配置するのが一般的)
            if os.path.exists(os.path.join(dist_dir, third_party_licenses_dir)):
                print(f"Including {third_party_licenses_dir} in .deb package")
                shutil.copytree(os.path.join(dist_dir, third_party_licenses_dir), 
                                os.path.join(doc_path, third_party_licenses_dir))

            if found_license:
                shutil.copy2(found_license, os.path.join(doc_path, "copyright"))

            control_content = f"""Package: {project_name}
Version: {version}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Developer <dev@example.com>
Depends: libwebkit2gtk-4.0-37, libgtk-3-0
Description: {project_name} backup tool
 A file backup utility using Wails.
 Includes hdiff, bzip2 and third-party licenses.
"""
            with open(os.path.join(deb_root, "DEBIAN/control"), "w") as f:
                f.write(control_content)

            subprocess.run(["dpkg-deb", "--build", deb_root, deb_file_path], check=True)
            print(f"Successfully created: {deb_file_path}")
            shutil.copy2(deb_file_path, dist_dir)
            
            if args.clean:
                shutil.rmtree(deb_root)
        except Exception as e:
            print(f"Skipping .deb creation: {e}")

    # 8. ZIP圧縮処理
    if args.zip:
        print(f"--- Archiving to {dist_dir}.zip ---")
        shutil.make_archive(dist_dir, 'zip', root_dir=".", base_dir=dist_dir)
        print(f"Archive created: {dist_dir}.zip")

    # 9. クリーンアップ
    if args.clean:
        print(f"Cleaning up source directory: {dist_dir}")
        if os.path.exists(dist_dir):
            shutil.rmtree(dist_dir)
        if os.path.exists(deb_file_path):
            os.remove(deb_file_path)

    print("--- Done! ---")

if __name__ == "__main__":
    create_release_package()
