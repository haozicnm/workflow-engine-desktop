// popup.js — 检查 native host 状态
async function checkStatus() {
  const statusEl = document.getElementById('status');
  
  try {
    const resp = await fetch('http://127.0.0.1:19527/health');
    const data = await resp.json();
    statusEl.textContent = '已连接 ✓';
    statusEl.className = 'value';
  } catch (e) {
    statusEl.textContent = '未连接 ✗';
    statusEl.className = 'value error';
  }
  
  // 检查已连接的标签页
  const bg = await chrome.runtime.getBackgroundPage();
  if (bg) {
    document.getElementById('tabs').textContent = bg.attachedTabs?.size || 0;
  }
}

checkStatus();
