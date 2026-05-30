/**
 * Workflow WebBridge — background.js (WebSocket 版)
 * 
 * 通过 WebSocket 与 workflow-engine 通信，无需 Native Messaging。
 * 扩展主动连接到 ws://localhost:19527/ws/browser
 * 
 * 优势：零配置，不需要安装 native messaging host
 */

// ═══════════════════════════════════════════════
// 配置
// ═══════════════════════════════════════════════

const WS_URL = 'ws://127.0.0.1:19527/ws/browser';
const RECONNECT_DELAY = 2000;
const COMMAND_TIMEOUT = 30000;

// ═══════════════════════════════════════════════
// 状态管理
// ═══════════════════════════════════════════════

const attachedTabs = new Set();
let activeTabId = null;
let lastFocusedTabId = null;
let ws = null;
let wsConnected = false;

// @e ref 系统
const refMap = new Map();
let refCounter = 1;

const REF_ROLES = new Set([
  'button', 'link', 'textbox', 'checkbox', 'radio',
  'combobox', 'listbox', 'menuitem', 'menuitemcheckbox',
  'menuitemradio', 'option', 'searchbox', 'slider',
  'spinbutton', 'switch', 'tab', 'treeitem',
]);

// ═══════════════════════════════════════════════
// WebSocket 通信
// ═══════════════════════════════════════════════

function connectWebSocket() {
  if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) {
    return;
  }
  
  try {
    ws = new WebSocket(WS_URL);
    
    ws.onopen = () => {
      wsConnected = true;
      console.log('[WebBridge] Connected to workflow-engine');
      // 注册自己
      ws.send(JSON.stringify({
        type: 'register',
        client: 'webbridge',
        version: '1.0.0',
        capabilities: Object.keys(tools),
      }));
    };
    
    ws.onmessage = async (event) => {
      try {
        const msg = JSON.parse(event.data);
        await handleCommand(msg);
      } catch (e) {
        console.error('[WebBridge] Failed to handle message:', e);
      }
    };
    
    ws.onclose = () => {
      wsConnected = false;
      console.log('[WebBridge] Disconnected, reconnecting...');
      setTimeout(connectWebSocket, RECONNECT_DELAY);
    };
    
    ws.onerror = (e) => {
      console.error('[WebBridge] WebSocket error:', e);
    };
  } catch (e) {
    console.error('[WebBridge] Failed to connect:', e);
    setTimeout(connectWebSocket, RECONNECT_DELAY);
  }
}

async function handleCommand(msg) {
  const { id, action, params = {} } = msg;
  
  if (!id || !action) return;
  
  try {
    if (!tools[action]) {
      throw new Error(`Unknown action: ${action}`);
    }
    
    const result = await tools[action](params);
    ws?.send(JSON.stringify({ id, success: true, data: result }));
  } catch (e) {
    ws?.send(JSON.stringify({ id, success: false, error: e.message }));
  }
}

// ═══════════════════════════════════════════════
// CDP 会话管理
// ═══════════════════════════════════════════════

async function ensureAttached(tabId) {
  if (attachedTabs.has(tabId)) {
    activeTabId = tabId;
    return;
  }
  try {
    await chrome.debugger.detach({ tabId });
  } catch {}
  await chrome.debugger.attach({ tabId }, '1.3');
  attachedTabs.add(tabId);
  activeTabId = tabId;
}

async function cdpCommand(method, params = {}) {
  if (activeTabId === null) {
    throw new Error('No tab attached. Call attach(tabId) first.');
  }
  return await chrome.debugger.sendCommand({ tabId: activeTabId }, method, params);
}

async function resolveTab() {
  if (activeTabId !== null) {
    try {
      const tab = await chrome.tabs.get(activeTabId);
      if (tab) return tab;
    } catch {
      attachedTabs.delete(activeTabId);
      activeTabId = null;
    }
  }
  if (lastFocusedTabId !== null) {
    try {
      const tab = await chrome.tabs.get(lastFocusedTabId);
      if (tab) return tab;
    } catch {
      lastFocusedTabId = null;
    }
  }
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id) throw new Error('No active tab found');
  lastFocusedTabId = tab.id;
  return tab;
}

// ═══════════════════════════════════════════════
// @e ref 系统
// ═══════════════════════════════════════════════

function resetRefs() {
  refMap.clear();
  refCounter = 1;
}

function makeRef(backendDOMNodeId, role, name) {
  const refId = `e${refCounter++}`;
  refMap.set(refId, { backendDOMNodeId, role, name });
  return refId;
}

function resolveRef(ref) {
  const id = ref.startsWith('@') ? ref.slice(1) : ref;
  return refMap.get(id);
}

function isRef(str) {
  return /^@?e\d+$/.test(str);
}

// ═══════════════════════════════════════════════
// 工具实现（与 Kimi WebBridge 完全兼容）
// ═══════════════════════════════════════════════

const tools = {
  // ─── 浏览器管理 ───
  
  async navigate(params) {
    let url = params.url;
    if (!url) throw new Error('navigate: url is required');
    
    const newTab = params.newTab;
    const tab = await resolveTab();
    
    if (newTab) {
      const created = await chrome.tabs.create({ url, active: true });
      lastFocusedTabId = created.id;
      await ensureAttached(created.id);
      await waitForLoad(created.id);
      return { success: true, url, tabId: created.id };
    }
    
    await ensureAttached(tab.id);
    await cdpCommand('Page.navigate', { url });
    await waitForLoad(tab.id);
    return { success: true, url: tab.url, tabId: tab.id };
  },

  async find_tab(params) {
    const { url, title, index } = params;
    const tabs = await chrome.tabs.query({});
    
    let target;
    if (index !== undefined) {
      target = tabs[index];
    } else if (url) {
      target = tabs.find(t => t.url?.includes(url));
    } else if (title) {
      target = tabs.find(t => t.title?.includes(title));
    }
    
    if (!target) throw new Error(`Tab not found: ${JSON.stringify(params)}`);
    lastFocusedTabId = target.id;
    await ensureAttached(target.id);
    return { success: true, tabId: target.id, url: target.url, title: target.title };
  },

  async list_tabs() {
    const tabs = await chrome.tabs.query({});
    return tabs.map((t, i) => ({
      index: i,
      tabId: t.id,
      url: t.url,
      title: t.title,
      active: t.active,
    }));
  },

  async close_tab(params) {
    const tabId = params.tabId || activeTabId;
    if (!tabId) throw new Error('No tab to close');
    await chrome.tabs.remove(tabId);
    attachedTabs.delete(tabId);
    if (activeTabId === tabId) activeTabId = null;
    return { success: true, closed: tabId };
  },

  async close_session() {
    for (const tabId of attachedTabs) {
      try { await chrome.debugger.detach({ tabId }); } catch {}
    }
    attachedTabs.clear();
    activeTabId = null;
    return { success: true, message: 'Session closed' };
  },

  // ─── 页面操作 ───

  async snapshot() {
    await ensureAttached((await resolveTab()).id);
    resetRefs();
    
    const axResult = await cdpCommand('Accessibility.getFullAXTree');
    const nodes = axResult.nodes || [];
    const tree = buildAxTree(nodes);
    
    const tab = await resolveTab();
    return {
      url: tab.url,
      title: tab.title,
      tree,
      refs: Object.fromEntries(
        [...refMap.entries()].map(([k, v]) => [k, { role: v.role, name: v.name }])
      ),
    };
  },

  async click(params) {
    const { selector } = params;
    if (!selector) throw new Error('click: selector is required');
    await ensureAttached((await resolveTab()).id);
    
    if (isRef(selector)) {
      return await clickByRef(selector);
    }
    return await clickBySelector(selector);
  },

  async fill(params) {
    const { selector, value } = params;
    if (!selector) throw new Error('fill: selector is required');
    if (value == null) throw new Error('fill: value is required');
    await ensureAttached((await resolveTab()).id);
    
    if (isRef(selector)) {
      return await fillByRef(selector, value);
    }
    return await fillBySelector(selector, value);
  },

  async evaluate(params) {
    const { code, expression } = params;
    const script = code || expression;
    if (!script) throw new Error('evaluate: code/expression is required');
    await ensureAttached((await resolveTab()).id);
    
    const result = await cdpCommand('Runtime.evaluate', {
      expression: script,
      returnByValue: true,
      awaitPromise: true,
    });
    
    if (result.exceptionDetails) {
      throw new Error(`evaluate error: ${result.exceptionDetails.text}`);
    }
    return { type: typeof result.result.value, value: result.result.value };
  },

  async screenshot(params) {
    await ensureAttached((await resolveTab()).id);
    const format = params.format || 'png';
    const quality = params.quality;
    
    const opts = { format };
    if (quality && format === 'jpeg') opts.quality = quality;
    
    const result = await cdpCommand('Page.captureScreenshot', opts);
    return { data: result.data, format };
  },

  async save_as_pdf(params) {
    await ensureAttached((await resolveTab()).id);
    const result = await cdpCommand('Page.printToPDF', {
      printBackground: true,
      preferCSSPageSize: true,
    });
    return { data: result.data };
  },

  // ─── 键盘/鼠标 ───

  async mouse_click(params) {
    const { selector, x, y } = params;
    await ensureAttached((await resolveTab()).id);
    
    let cx, cy;
    if (x !== undefined && y !== undefined) {
      cx = x; cy = y;
    } else if (selector) {
      const objectId = isRef(selector)
        ? await objectIdFromRef(selector)
        : await objectIdFromSelector(selector);
      
      await cdpCommand('Runtime.callFunctionOn', {
        objectId,
        functionDeclaration: "function() { this.scrollIntoView({block:'center',inline:'center'}); }",
      });
      
      const box = await cdpCommand('DOM.getBoxModel', { objectId });
      const c = box.model?.content;
      if (!c || c.length < 8) throw new Error('Element has no layout box');
      cx = (c[0] + c[2] + c[4] + c[6]) / 4;
      cy = (c[1] + c[3] + c[5] + c[7]) / 4;
    } else {
      throw new Error('mouse_click: need selector or x,y coordinates');
    }
    
    await cdpCommand('Input.dispatchMouseEvent', { type: 'mouseMoved', x: cx, y: cy, button: 'none', buttons: 0 });
    await cdpCommand('Input.dispatchMouseEvent', { type: 'mousePressed', x: cx, y: cy, button: 'left', buttons: 1, clickCount: 1 });
    await cdpCommand('Input.dispatchMouseEvent', { type: 'mouseReleased', x: cx, y: cy, button: 'left', buttons: 0, clickCount: 1 });
    
    return { success: true, x: Math.round(cx), y: Math.round(cy) };
  },

  async key_type(params) {
    const { text, delay } = params;
    if (!text) throw new Error('key_type: text is required');
    await ensureAttached((await resolveTab()).id);
    
    for (const char of text) {
      await cdpCommand('Input.dispatchKeyEvent', {
        type: 'keyDown', text: char, key: char,
      });
      await cdpCommand('Input.dispatchKeyEvent', {
        type: 'keyUp', key: char,
      });
      if (delay) await sleep(delay);
    }
    return { success: true, typed: text.length };
  },

  async send_keys(params) {
    const { key, modifiers } = params;
    if (!key) throw new Error('send_keys: key is required');
    await ensureAttached((await resolveTab()).id);
    
    const mods = {};
    if (modifiers?.includes('ctrl')) mods.ctrl = true;
    if (modifiers?.includes('shift')) mods.shift = true;
    if (modifiers?.includes('alt')) mods.alt = true;
    if (modifiers?.includes('meta')) mods.meta = true;
    
    await cdpCommand('Input.dispatchKeyEvent', {
      type: 'keyDown', key, ...mods,
    });
    await cdpCommand('Input.dispatchKeyEvent', {
      type: 'keyUp', key, ...mods,
    });
    return { success: true, key, modifiers };
  },

  // ─── 网络 ───

  async network(params) {
    await ensureAttached((await resolveTab()).id);
    const action = params.action || 'enable';
    
    if (action === 'enable') {
      await cdpCommand('Network.enable');
      return { success: true, enabled: true };
    }
    if (action === 'disable') {
      await cdpCommand('Network.disable');
      return { success: true, enabled: false };
    }
    throw new Error(`network: unknown action "${action}"`);
  },

  async upload(params) {
    const { selector, filePaths } = params;
    if (!selector) throw new Error('upload: selector is required');
    if (!filePaths?.length) throw new Error('upload: filePaths is required');
    await ensureAttached((await resolveTab()).id);
    
    const result = await cdpCommand('Runtime.evaluate', {
      expression: `document.querySelector(${JSON.stringify(selector)})`,
      returnByValue: false,
    });
    if (result.result?.subtype === 'null') {
      throw new Error(`upload: element not found: ${selector}`);
    }
    
    const { backendNodeId } = await cdpCommand('DOM.describeNode', {
      objectId: result.result.objectId,
    });
    
    await cdpCommand('DOM.setFileInputFiles', {
      files: filePaths,
      backendNodeId,
    });
    
    return { success: true, files: filePaths.length };
  },

  async cdp(params) {
    const { method, params: cdpParams } = params;
    if (!method) throw new Error('cdp: method is required');
    await ensureAttached((await resolveTab()).id);
    return await cdpCommand(method, cdpParams || {});
  },
};

// ═══════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function waitForLoad(tabId, timeout = 30000) {
  return new Promise((resolve) => {
    const listener = (id, info) => {
      if (id === tabId && info.status === 'complete') {
        chrome.tabs.onUpdated.removeListener(listener);
        resolve();
      }
    };
    chrome.tabs.onUpdated.addListener(listener);
    setTimeout(() => {
      chrome.tabs.onUpdated.removeListener(listener);
      resolve();
    }, timeout);
  });
}

function buildAxTree(nodes) {
  const nodeMap = new Map();
  for (const n of nodes) nodeMap.set(n.nodeId, n);
  
  function formatNode(node) {
    const role = typeof node.role === 'object' ? node.role?.value : node.role;
    
    if (!role || role === 'none' || role === 'generic') {
      const children = [];
      for (const cid of (node.childIds || [])) {
        const child = nodeMap.get(cid);
        if (child) {
          const result = formatNode(child);
          if (result) {
            if (Array.isArray(result)) children.push(...result);
            else children.push(result);
          }
        }
      }
      return children.length === 1 ? children[0] : children.length > 0 ? children : null;
    }
    
    const entry = { role };
    const name = typeof node.name === 'object' ? node.name?.value : node.name;
    if (name) entry.name = name;
    const value = typeof node.value === 'object' ? node.value?.value : node.value;
    if (value) entry.value = value;
    
    if (REF_ROLES.has(role) && node.backendDOMNodeId != null) {
      entry.ref = `@${makeRef(node.backendDOMNodeId, role, name || '')}`;
    }
    
    const children = [];
    for (const cid of (node.childIds || [])) {
      const child = nodeMap.get(cid);
      if (child) {
        const result = formatNode(child);
        if (result) {
          if (Array.isArray(result)) children.push(...result);
          else children.push(result);
        }
      }
    }
    if (children.length > 0) entry.children = children;
    
    return entry;
  }
  
  if (nodes.length === 0) return [];
  const result = formatNode(nodes[0]);
  return result ? (Array.isArray(result) ? result : [result]) : [];
}

async function objectIdFromRef(ref) {
  const info = resolveRef(ref);
  if (!info) throw new Error(`Unknown ref "${ref}". Run snapshot first.`);
  const { object } = await cdpCommand('DOM.resolveNode', {
    backendNodeId: info.backendDOMNodeId,
  });
  if (!object?.objectId) throw new Error(`Could not resolve ref "${ref}"`);
  return object.objectId;
}

async function objectIdFromSelector(selector) {
  const result = await cdpCommand('Runtime.evaluate', {
    expression: `document.querySelector(${JSON.stringify(selector)})`,
    returnByValue: false,
  });
  if (result.result?.subtype === 'null') {
    throw new Error(`Element not found: ${selector}`);
  }
  return result.result.objectId;
}

async function clickByRef(ref) {
  const objectId = await objectIdFromRef(ref);
  const info = resolveRef(ref);
  
  await cdpCommand('Runtime.callFunctionOn', {
    objectId,
    functionDeclaration: "function() { this.scrollIntoView({block:'center',inline:'center'}); }",
  });
  
  const box = await cdpCommand('DOM.getBoxModel', { objectId });
  const c = box.model?.content;
  if (!c || c.length < 8) throw new Error(`Ref "${ref}" has no visible area`);
  
  const cx = (c[0] + c[2] + c[4] + c[6]) / 4;
  const cy = (c[1] + c[3] + c[5] + c[7]) / 4;
  
  await cdpCommand('Input.dispatchMouseEvent', { type: 'mouseMoved', x: cx, y: cy, button: 'none', buttons: 0 });
  await cdpCommand('Input.dispatchMouseEvent', { type: 'mousePressed', x: cx, y: cy, button: 'left', buttons: 1, clickCount: 1 });
  await cdpCommand('Input.dispatchMouseEvent', { type: 'mouseReleased', x: cx, y: cy, button: 'left', buttons: 0, clickCount: 1 });
  
  return { success: true, ref, role: info.role, name: info.name, x: Math.round(cx), y: Math.round(cy) };
}

async function clickBySelector(selector) {
  const result = await cdpCommand('Runtime.evaluate', {
    expression: `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return { error: 'element not found: ${selector}' };
      el.scrollIntoView({ block: 'center' });
      el.click();
      return { success: true, tag: el.tagName, text: el.textContent?.slice(0, 100) };
    })()`,
    returnByValue: true,
  });
  if (result.exceptionDetails) throw new Error(`click: ${result.exceptionDetails.text}`);
  const value = result.result.value;
  if (value?.error) throw new Error(value.error);
  return value || { success: true };
}

async function fillByRef(ref, value) {
  const objectId = await objectIdFromRef(ref);
  
  const jsCode = `function() {
    const el = this;
    el.focus();
    if (el.isContentEditable) {
      const sel = window.getSelection();
      if (sel) {
        const range = document.createRange();
        range.selectNodeContents(el);
        sel.removeAllRanges();
        sel.addRange(range);
      }
      let inserted = false;
      try { inserted = document.execCommand('insertText', false, ${JSON.stringify(value)}); } catch(_e) { inserted = false; }
      if (!inserted) {
        el.textContent = ${JSON.stringify(value)};
        el.dispatchEvent(new InputEvent('input', {inputType:'insertText', data:${JSON.stringify(value)}, bubbles:true}));
      }
      return {success:true, tag:el.tagName, mode:'contenteditable'};
    }
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')?.set
      || Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value')?.set;
    if (setter) setter.call(el, ${JSON.stringify(value)});
    else el.value = ${JSON.stringify(value)};
    el.dispatchEvent(new Event('input', {bubbles:true}));
    el.dispatchEvent(new Event('change', {bubbles:true}));
    return {success:true, tag:el.tagName, mode:'value'};
  }`;
  
  const result = await cdpCommand('Runtime.callFunctionOn', {
    objectId,
    functionDeclaration: jsCode,
    returnByValue: true,
  });
  
  if (result.exceptionDetails) {
    throw new Error(`fill: ${result.exceptionDetails.text}`);
  }
  return result.result.value || { success: true };
}

async function fillBySelector(selector, value) {
  const result = await cdpCommand('Runtime.evaluate', {
    expression: `(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return { error: 'element not found: ${selector}' };
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')?.set;
      if (setter) setter.call(el, ${JSON.stringify(value)});
      else el.value = ${JSON.stringify(value)};
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.dispatchEvent(new Event('change', { bubbles: true }));
      return { success: true, tag: el.tagName };
    })()`,
    returnByValue: true,
    awaitPromise: false,
  });
  if (result.exceptionDetails) throw new Error(`fill: ${result.exceptionDetails.text}`);
  const val = result.result.value;
  if (val?.error) throw new Error(val.error);
  return val || { success: true };
}

// ═══════════════════════════════════════════════
// 事件监听
// ═══════════════════════════════════════════════

chrome.tabs.onRemoved.addListener((tabId) => {
  attachedTabs.delete(tabId);
  if (activeTabId === tabId) activeTabId = null;
  if (lastFocusedTabId === tabId) lastFocusedTabId = null;
});

chrome.debugger.onDetach.addListener((source) => {
  if (source.tabId) {
    attachedTabs.delete(source.tabId);
    if (activeTabId === source.tabId) activeTabId = null;
  }
});

chrome.tabs.onActivated.addListener((info) => {
  lastFocusedTabId = info.tabId;
});

// ═══════════════════════════════════════════════
// 启动
// ═══════════════════════════════════════════════

console.log('[WebBridge] Background loaded');
connectWebSocket();
