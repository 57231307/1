/**
 * 安卓壳 gateway 卡页（服务器地址设置）的决策函数与页面装配。
 *
 * 调用方 mobile-app/dist/index.html 以普通脚本引入本文件，页内标记依赖的 id 为
 * form url err saved savedText enterBtn changeBtn perm permBtn permMsg。
 * 传给谁，三个决策纯函数同时供 frontend 单测经相对路径引用（启动视图、探测结果、
 * 保存并进入三段判断都在这里单点定义，装配层只执行结果），
 * 壳内能力经 window.__TAURI__ 调 remember_gateway 与 probe_server_reachability 与 reopen_gateway。
 * 存什么 仅存规范化后的服务器地址（origin 加路径），不存凭证。
 * 存哪里 本 origin 的 localStorage，键 bingxi_server_url。
 */
(function () {
  'use strict';

  var KEY = 'bingxi_server_url';

  /** 规范化手填地址，去首尾空白，缺协议补 https，仅收 http/https，返回 origin 加路径或空串。 */
  function normalized(raw) {
    var v = (raw || '').trim();
    if (!v) return '';
    if (!/^https?:\/\//i.test(v)) v = 'https://' + v;
    try {
      var u = new URL(v);
      if (u.protocol !== 'http:' && u.protocol !== 'https:') return '';
      return u.origin + (u.pathname === '/' ? '' : u.pathname);
    } catch (e) {
      return '';
    }
  }

  /**
   * 启动视图决策，纯函数。入参 reconfigureRequested 为地址 query 是否带 reconfigure=1
   * （该标记由壳内 reopen_gateway 追加，代表两条失败路径，登录不可达或连续凭证失败），
   * hasSaved 为本地是否存过地址，probeSupported 为壳内探测桥是否可用。
   * 返回 manual-empty，空表单且不带旧地址且不自动跳转，
   * auto-probe，先探测已存地址再定进入或回表单，
   * saved-card，无桥的桌面预览保留手动进入或更改，
   * first-install，首装空表单直接手填。
   */
  function decideStartup(input) {
    if (input.reconfigureRequested) return 'manual-empty';
    if (!input.hasSaved) return 'first-install';
    if (!input.probeSupported) return 'saved-card';
    return 'auto-probe';
  }

  /**
   * 探测结果决策，纯函数。入参 outcome 为壳内探测结论，reachable 表示 TCP 握手成功，
   * dns、refused、timeout、unreachable 为可区分的失败种类。
   * 通过则自动进入且全程不露出地址输入，不通则回空表单等待用户改填，不自动跳回旧地址。
   * 探测成功只代表端口能握手上，不等于页面可用，随后登录仍失败时由远端登录页自己的
   * 连续失败计数处理，卡页与远端页面 origin 不同存储不互通，卡页不清那份计数。
   */
  function decideProbeOutcome(input) {
    return input.outcome === 'reachable' ? 'navigate' : 'manual-empty';
  }

  /**
   * 探测失败提示映射，纯函数。按结论种类给能指引下一步的文案，
   * 解析失败与端口拒绝不糊成同一句连接失败，reachable 与未知结论返回空串。
   */
  function probeMessage(outcome) {
    if (outcome === 'dns') return '这台服务器的域名解析不出来，请检查地址是否拼写正确';
    if (outcome === 'refused') return '服务器拒绝了连接，请确认服务已启动并且端口填写正确';
    if (outcome === 'timeout') return '连接服务器超时，请检查手机网络与服务器状态后重试';
    if (outcome === 'unreachable') return '连不上这台服务器，请检查网络与服务器状态后重试';
    return '';
  }

  /**
   * 保存并进入决策，纯函数。入参 valid 为地址规范化是否通过，
   * probeSupported 为壳内探测桥是否可用，outcome 为探测结论。
   * 地址非法回 invalid，无桥的桌面预览沿用旧行为直接跳转，
   * 壳内探测不通回 unreachable，此时不落库不跳转，避免把错地址存成下次启动的自动目标。
   */
  function decideSubmit(input) {
    if (!input.valid) return 'invalid';
    if (!input.probeSupported) return 'navigate';
    return input.outcome === 'reachable' ? 'navigate' : 'unreachable';
  }

  var api = {
    KEY: KEY,
    normalized: normalized,
    decideStartup: decideStartup,
    decideProbeOutcome: decideProbeOutcome,
    probeMessage: probeMessage,
    decideSubmit: decideSubmit,
  };
  if (typeof globalThis !== 'undefined') globalThis.BingxiGateway = api;
  if (typeof module !== 'undefined' && module.exports) module.exports = api;

  if (typeof document === 'undefined') return;

  function boot() {
    var form = document.getElementById('form');
    var urlInput = document.getElementById('url');
    var err = document.getElementById('err');
    var savedWrap = document.getElementById('saved');
    var savedText = document.getElementById('savedText');
    var enterBtn = document.getElementById('enterBtn');
    var changeBtn = document.getElementById('changeBtn');
    if (!form || !urlInput || !err || !savedWrap || !savedText || !enterBtn || !changeBtn) return;

    var bridge = (typeof window !== 'undefined' && window.__TAURI__ && window.__TAURI__.core) || null;
    var probeSupported = !!(bridge && bridge.invoke);

    function go(target) {
      window.location.replace(target);
    }

    /** 记录本页稳定基址供登录失败回弹，壳侧剥掉 query 后存住，失败只留痕不阻塞。 */
    function rememberSelf() {
      if (!probeSupported) return;
      bridge
        .invoke('remember_gateway', { url: window.location.href })
        .catch(function () {
          err.textContent = '本页地址记录失败，登录失败时将无法自动回到此页';
        });
    }

    /** 壳内 TCP 可达性探测，入参为规范化后的服务器地址，返回结论字符串的 Promise，桥缺失返回 null。 */
    function probe(target) {
      if (!probeSupported) return null;
      return bridge.invoke('probe_server_reachability', { target: target });
    }

    function showEmptyForm(message) {
      savedWrap.style.display = 'none';
      form.style.display = '';
      urlInput.value = '';
      err.textContent = message || '';
      urlInput.focus();
    }

    function showSavedCard(saved) {
      form.style.display = 'none';
      savedWrap.style.display = '';
      savedText.textContent = '上次地址：' + saved;
      enterBtn.onclick = function () {
        go(saved);
      };
      changeBtn.onclick = function () {
        showEmptyForm('');
      };
    }

    function setupPermissions() {
      if (!probeSupported) return;
      var wrap = document.getElementById('perm');
      var btn = document.getElementById('permBtn');
      var msg = document.getElementById('permMsg');
      if (!wrap || !btn || !msg) return;
      wrap.style.display = '';
      btn.onclick = function () {
        btn.disabled = true;
        msg.style.color = '';
        msg.textContent = '正在申请，请在系统弹窗中选择允许';
        Promise.allSettled([
          bridge.invoke('app_request_camera_permission'),
          bridge.invoke('bluetooth_request_permission'),
        ]).then(function (results) {
          function granted(result) {
            return result.status === 'fulfilled' && result.value && result.value.granted === true;
          }
          var camera = granted(results[0]);
          var bluetooth = granted(results[1]);
          msg.style.color = camera && bluetooth ? '#15803d' : '#b45309';
          msg.textContent =
            '相机' +
            (camera ? '已授权' : '未授权') +
            '，蓝牙' +
            (bluetooth ? '已授权' : '未授权') +
            '，可随时在系统设置中调整';
          btn.disabled = false;
        });
      };
    }

    function submitHandler(event) {
      event.preventDefault();
      var ok = normalized(urlInput.value);
      var verdict = api.decideSubmit({ valid: !!ok, probeSupported: probeSupported });
      if (verdict === 'invalid') {
        err.textContent = '地址无效，请填写含 http/https 的完整地址';
        return;
      }
      if (verdict === 'navigate') {
        finishSave(ok);
        return;
      }
      var pending = probe(ok);
      if (!pending) {
        finishSave(ok);
        return;
      }
      err.textContent = '正在确认服务器可达…';
      pending
        .then(function (outcome) {
          var verdict = api.decideSubmit({ valid: true, probeSupported: true, outcome: outcome });
          if (verdict === 'navigate') {
            finishSave(ok);
          } else {
            err.textContent = api.probeMessage(outcome) || '该地址连不上，请检查服务器状态或更换地址';
          }
        })
        .catch(function () {
          err.textContent = '该地址连不上，请检查服务器状态或更换地址';
        });
    }

    /** 探测放行后落库并跳转，存储不可用不阻塞进入。 */
    function finishSave(ok) {
      err.textContent = '';
      try {
        localStorage.setItem(KEY, ok);
      } catch (e) {
        // 隐私模式下存储不可用，本次进入仍然有效，仅下次启动需重填
      }
      go(ok);
    }

    function startup() {
      rememberSelf();
      setupPermissions();
      var reconfigure = /[?&]reconfigure=1(&|$)/.test(window.location.search);
      var saved = '';
      try {
        saved = localStorage.getItem(KEY) || '';
      } catch (e) {
        // 隐私模式下读不到即按未存过处理，走空表单
      }
      var view = api.decideStartup({
        reconfigureRequested: reconfigure,
        hasSaved: !!saved,
        probeSupported: probeSupported,
      });
      if (view === 'manual-empty') {
        showEmptyForm('上次没有成功进入系统，请确认服务器地址后重新填写');
        return;
      }
      if (view === 'first-install') {
        showEmptyForm('');
        return;
      }
      if (view === 'saved-card') {
        showSavedCard(saved);
        return;
      }
      var pending = probe(saved);
      if (!pending) {
        showSavedCard(saved);
        return;
      }
      pending
        .then(function (outcome) {
          if (api.decideProbeOutcome({ outcome: outcome }) === 'navigate') {
            go(saved);
          } else {
            showEmptyForm(api.probeMessage(outcome) || '无法连接上次保存的服务器，请重新填写地址');
          }
        })
        .catch(function () {
          showEmptyForm('无法确认上次保存的地址可用，请重新填写地址');
        });
    }

    form.onsubmit = submitHandler;
    startup();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else {
    boot();
  }
})();
