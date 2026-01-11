/*
import {
  SelectAnyFile,
  SelectBackupFolder,
  GetFileSize,
  WriteTextFile,
  ReadTextFile,
  RestoreBackup
} from '../wailsjs/go/main/App';
*/
import {
  SelectAnyFile,
  SelectBackupFolder,
  GetFileSize,
  WriteTextFile,
  ReadTextFile,
  RestoreBackup,
  EventsOn
} from './tauri_exports';

import {
  i18n,
  getActiveTab,
  addToRecentFiles,
  saveCurrentSession
} from './state';

import {
  renderTabs,
  UpdateDisplay,
  UpdateHistory,
  toggleProgress,
  showFloatingMessage
} from './ui';

import {
  addTab,
  OnExecute,
  updateExecute,
} from './actions';
import { ask } from '@tauri-apps/plugin-dialog';



// --- ドラッグアンドドロップの基本防止設定 ---
const preventDefault = (e) => {
  e.preventDefault();
  e.stopPropagation();
};

export function setupGlobalEvents() {
  // Only this
  window.addEventListener('dragenter', preventDefault, true);

  // --- クリックイベントリスナー ---
  window.addEventListener('click', async (e) => {
    // 修正ポイント：ターゲットがボタン内部のアイコン等の場合でも正しくIDを拾う
    const target = e.target.closest('button') || e.target;
    const id = target.id;
    const tab = getActiveTab();
    
    if (id === 'add-tab-btn') { 
      addTab(); 
      return; 
    }

    const noteBtn = e.target.closest('.note-btn');
    if (noteBtn) {
      const path = noteBtn.getAttribute('data-path');
      const cur = await ReadTextFile(path + ".note").catch(() => "");
      const val = prompt("Memo:", cur);
      if (val !== null) { 
        await WriteTextFile(path + ".note", val); 
        UpdateHistory(); 
      }
      return;
    }

    if (id === 'workfile-btn' || id === 'compact-workfile-btn') {
      const res = await SelectAnyFile(i18n.workFileBtn, [{ DisplayName: "Work file", Pattern: "*.*" }]);
      if (res) {
        tab.workFile = res;
        tab.workFileSize = await GetFileSize(res);
        addToRecentFiles(res);
        renderTabs(); UpdateDisplay(); UpdateHistory();
        saveCurrentSession();
        showFloatingMessage(i18n.updatedWorkFile);
      }
      return; // return追加で後続判定を防止
    } else if (id === 'backupdir-btn') {
      const res = await SelectBackupFolder();
      if (res) {
        tab.backupDir = res;
        UpdateDisplay(); UpdateHistory();
        saveCurrentSession();
        showFloatingMessage(i18n.updatedBackupDir);
      }
      return;
    } else if (id === 'execute-backup-btn' || id === 'compact-execute-btn') {
      OnExecute();
      return;
    } else if (id === 'refresh-diff-btn') {
      UpdateHistory();
      return;
    } else if (id === 'select-all-btn') {
      const cbs = document.querySelectorAll('.diff-checkbox');
      const all = Array.from(cbs).every(cb => cb.checked);
      cbs.forEach(cb => cb.checked = !all);
      return;
    } else if (id === 'apply-selected-btn') {
      // --- 物理ガード開始 ---
      e.preventDefault();
      e.stopPropagation();

      const targets = Array.from(document.querySelectorAll('.diff-checkbox:checked')).map(el => el.value);
      
      // ターゲットがない場合は即終了
      if (targets.length === 0) return;

      // 【重要修正】confirm を await ask に変更
      // これによりユーザーが「はい」を押すまで、JSもRustもここで完全に一時停止します
      const isConfirmed = await ask(i18n.restoreConfirm, { 
        title: 'CG File Backup',
        type: 'warning' 
      });
      
      if (isConfirmed) {
        toggleProgress(true, "Restoring...");
        try { 
          // 復元処理。awaitにより一つずつ順番に完了を待ちます
          for (const p of targets) { 
            await RestoreBackup(p, tab.workFile); 
          } 
          toggleProgress(false); 
          // 処理がすべて終わってからメッセージを表示
          showFloatingMessage(i18n.diffApplySuccess); 
          UpdateHistory(); 
        }
        catch (err) { 
          toggleProgress(false); 
          alert(err); 
        }
      }
      // 他のID判定（OnExecute等）に流れないよう、ここで完全に終了させる
      return; 
    }
  });

  // --- 変更イベントリスナー ---
  document.addEventListener('change', (e) => {
    const id = e.target.id;
    const name = e.target.name;
    const value = e.target.value;
    if (['backupMode', 'archive-format'].includes(e.target.name) || id === 'archive-format') {
      UpdateDisplay();
    }
    if (['backupMode', 'archive-format'].includes(name) || id === 'archive-format' || id === 'diff-algo') {
      UpdateDisplay();
      updateExecute();
    }
    if (id === 'compact-mode-select') {
      const radio = document.querySelector(`input[name="backupMode"][value="${value}"]`);
      if (radio) { 
        radio.checked = true; 
        UpdateDisplay(); 
        updateExecute();
      }
    }
  });

  EventsOn("compact-mode-event", (isCompact) => {
    const view = document.getElementById("compact-view");
    if (isCompact) {
      document.body.classList.add("compact-mode");
      if (view) view.classList.remove("hidden");
      if (typeof UpdateDisplay === 'function') {
        UpdateDisplay();
      }
    } else {
      document.body.classList.remove("compact-mode");
      if (view) view.classList.add("hidden");
    }
  });
}
