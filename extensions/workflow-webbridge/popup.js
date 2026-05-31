const DEFAULT_PORT = 19529;
const portInput = document.getElementById('port');
const saveBtn = document.getElementById('save');
const msg = document.getElementById('msg');

// 加载当前配置
chrome.storage.sync.get(['wfPort'], (result) => {
    portInput.value = result.wfPort || DEFAULT_PORT;
});

// 保存
saveBtn.addEventListener('click', () => {
    const port = parseInt(portInput.value, 10);
    if (port < 1024 || port > 65535) {
        msg.textContent = '端口范围: 1024-65535';
        msg.className = 'status disconnected';
        return;
    }
    chrome.storage.sync.set({ wfPort: port }, () => {
        msg.textContent = `已保存，正在连接 :${port}...`;
        msg.className = 'status connected';
    });
});

// 检查连接状态
chrome.runtime.sendMessage({ type: 'getStatus' }, (resp) => {
    if (resp && resp.connected) {
        msg.textContent = `已连接 :${resp.port}`;
        msg.className = 'status connected';
    } else {
        msg.textContent = `未连接 :${portInput.value}`;
        msg.className = 'status disconnected';
    }
});
