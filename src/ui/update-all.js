import { renderRecentFiles } from "./recent.js";
import { renderTabs } from "./tabs.js";
import { UpdateDisplay } from "./display.js";
import { UpdateHistory } from "./history.js";

export async function UpdateAllUI() {
  renderRecentFiles();
  renderTabs();
  await UpdateDisplay();
  UpdateHistory();
}
