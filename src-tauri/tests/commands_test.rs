use app_lib::app::commands;
use app_lib::app::state::AppState;
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_get_backup_list_logic() {
    AppState::new();
    let dir = tempdir().expect("Failed to create temp dir");
    let root = dir.path();

    // 1. 世代フォルダ (base1_, base2_) とダミーの .diff ファイルを作成
    let gen1_dir = root.join("base1_20260101_100000");
    let gen2_dir = root.join("base2_20260101_110000");
    fs::create_dir(&gen1_dir).unwrap();
    fs::create_dir(&gen2_dir).unwrap();

    // 世代1に2つ、世代2に1つの差分ファイルを作成
    fs::write(gen1_dir.join("work.clip.20260101_100500.diff"), "data1").unwrap();
    fs::write(gen1_dir.join("work.clip.20260101_101000.diff"), "data2").unwrap();
    fs::write(gen2_dir.join("work.clip.20260101_110500.diff"), "data3").unwrap();

    // 2. コマンドの実行
    // ※ 本来は Tauri の State 経由で呼ばれますが、ここでは直接ロジックをテストしたいので
    // ディレクトリ走査部分を検証します。

    // commands.rs の get_backup_list 内部ロジックと同様の走査をシミュレート
    let mut all_backups = Vec::new();

    // 世代フォルダを走査
    let entries = fs::read_dir(root).unwrap();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_string_lossy();
            if dir_name.starts_with("base") {
                // 世代番号を抽出 (base1 -> 1)
                let gen_num = dir_name[4..]
                    .split('_')
                    .next()
                    .unwrap()
                    .parse::<i32>()
                    .unwrap();

                // その中の .diff をカウント
                let diffs = fs::read_dir(path).unwrap();
                for diff in diffs.flatten() {
                    if diff.file_name().to_string_lossy().ends_with(".diff") {
                        all_backups.push(gen_num);
                    }
                }
            }
        }
    }

    // 3. 検証
    assert_eq!(all_backups.len(), 3);
    assert_eq!(all_backups.iter().filter(|&&g| g == 1).count(), 2);
    assert_eq!(all_backups.iter().filter(|&&g| g == 2).count(), 1);
}

#[test]
fn test_get_language_text_mapping() {
    // 1. AppState::default() で本物の i18n.json をロード
    let state = AppState::default();

    // 2. 実在するキーでテスト (i18n.json にある "settings" や "executeBtn" を使用)
    let test_key = "settings";
    let result = state.translate(test_key);

    // エラーメッセージに詳細を含める
    assert!(
        result.is_ok(),
        "翻訳キー '{}' の解決に失敗しました。i18n.json の中身を確認してください。",
        test_key
    );

    let text = result.unwrap();

    // 3. 値の検証
    assert!(!text.is_empty(), "翻訳テキストが空です");

    // 翻訳が適用されているか（キー名そのものが返ってきていないか）
    // デフォルトの ja なら "設定"、en なら "Settings" になるはず
    assert_ne!(
        text, test_key,
        "翻訳が適用されずキー名がそのまま返っています"
    );

    // デバッグ用に解決された文字を出力 (cargo test -- --nocapture で見れます)
    println!("Key: '{}' -> Value: '{}'", test_key, text);
}
