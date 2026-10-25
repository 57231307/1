/**
 * 安卓壳 gateway 卡页（服务器地址设置页）的决策函数与页面装配。
 *
 * 调用方 mobile-app/dist/index.html 以普通脚本引入本文件，页内标记依赖的 id 为
 * form url err saved savedText enterBtn changeBtn retry retryText retryBtn retryChangeBtn。
 * 传给谁，五个决策纯函数同时供 frontend 单测经相对路径引用（启动视图、连续失败推进、
 * 探测结论、提示句、可见视图模型五段判断都在这里单点定义，装配层只执行结果），
 * 壳内能力经 window.__TAURI__ 调 remember_gateway 与 probe_server_reachability。
 * 免重复配置的口径，配过一次地址后每次启动先探测，可达即自动进入；不可达只给重试面板，
 * 连续失败计满 5 次才把配置窗拿出来问一次，且问的时候不回填已配置的地址。
 * 存什么 规范化后的服务器地址（origin 加路径）与连续连接失败次数，不存凭证。
 * 存哪里 本 origin 的 localStorage，键 bingxi_server_url 与 bingxi_connect_fail。
 * 计数为什么存本地而不是存壳内 Rust，Rust 侧的进程内状态随应用退出即丢，跨启动的
 * "连续"必须落在持久层，而卡页 origin 的 localStorage 正是壳内唯一跨启动存活的可写位置。
 */
(function () {
  'use strict';

  var KEY = 'bingxi_server_url';
  var FAIL_KEY = 'bingxi_connect_fail';
  /** 连续连接失败达到该次数才重新询问地址，5 次是"多半地址不对"与"服务器临时不巧"的分界。 */
  var FAIL_LIMIT = 5;

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
   * 连续失败次数的存储值解析，纯函数。空串、非数字、负数与损坏值都按 0 次。
   * 计数只决定第几次才重新询问，读不出来时按"还没失败过"处理，最多是晚几轮再问，
   * 不会把已配好的地址当成没配过。
   */
  function parseConnectFailures(raw) {
    if (raw === null || raw === undefined || raw === '') return 0;
    var n = parseInt(String(raw), 10);
    if (isNaN(n) || n < 0) return 0;
    return n;
  }

  /**
   * 连续失败计数推进，纯函数。入参 prev 为本次之前的次数，outcome 为壳内探测结论。
   * 可达即归零（"连续"的定义是中间没有成功过），不可达在原值上加一。
   */
  function nextConnectFailures(prev, outcome) {
    return outcome === 'reachable' ? 0 : prev + 1;
  }

  /**
   * 启动视图决策，纯函数。入参 reconfigureRequested 为地址 query 是否带 reconfigure=1
   * （该标记由壳内 reopen_gateway 追加，代表远端登录页已经数过它自己的连续失败），
   * hasSaved 为本地是否存过地址，probeSupported 为壳内探测桥是否可用。
   * 返回 manual-empty，空表单且不带旧地址且不自动跳转，
   * auto-probe，先探测已存地址再定进入还是重试，
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
   * 探测结果决策，纯函数。入参 outcome 为壳内探测结论，failures 为本次累加后的连续失败次数。
   * 可达返回 navigate，自动进入且全程不露出地址输入；不可达在未计满 FAIL_LIMIT 时返回 retry，
   * 只给重试面板不出配置窗（第二次启动不因一次连不上就要求重配）；
   * 计满 FAIL_LIMIT 才返回 manual-empty 重新询问，此时也不回填已配置的地址。
   */
  function decideProbeOutcome(input) {
    if (input.outcome === 'reachable') return 'navigate';
    return input.failures >= FAIL_LIMIT ? 'manual-empty' : 'retry';
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
   * 不可达时的页内提示句，纯函数。入参 view 取 retry 或 manual-empty，reason 为探测结论
   * （空串表示本轮没有探测结论，例如远端登录页按回弹标记主动要求重填）。
   * 只说下一步该做什么，不报端口、不报结论码，也不把已存的地址显示出来。
   */
  function connectHint(view, reason) {
    var cause = probeMessage(reason) || '连不上这台服务器';
    if (view === 'manual-empty') {
      return cause + '，已多次连不上，请确认地址是否正确或改用其他服务器地址';
    }
    return cause + '，可以稍后再试一次，也可以改用其他服务器地址';
  }

  /**
   * 卡页可见视图模型，纯函数。入参 view 取 probing、first-install、manual-empty、retry、
   * saved-card 之一，saved 为存档地址，message 为提示句。
   * 返回六项可见位与文本，装配层只把它们写到元素上，不再各自决定谁显示。
   * first-install 与 manual-empty 的 formValue 恒为空串，这是"重新询问不得回填已配置地址"
   * 的单点定义；存档地址只出现在 saved-card 的说明文本里，任何视图下都不进输入框。
   */
  function buildConnectView(input) {
    var view = input.view;
    var message = input.message || '';
    if (view === 'saved-card') {
      return {
        savedVisible: true,
        savedText: '上次地址：' + input.saved,
        formVisible: false,
        formValue: '',
        retryVisible: false,
        retryText: '',
        errorText: '',
      };
    }
    if (view === 'retry') {
      return {
        savedVisible: false,
        savedText: '',
        formVisible: false,
        formValue: '',
        retryVisible: true,
        retryText: message,
        errorText: '',
      };
    }
    if (view === 'probing') {
      return {
        savedVisible: false,
        savedText: '',
        formVisible: false,
        formValue: '',
        retryVisible: false,
        retryText: '',
        errorText: message,
      };
    }
    return {
      savedVisible: false,
      savedText: '',
      formVisible: true,
      formValue: '',
      retryVisible: false,
      retryText: '',
      errorText: message,
    };
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
    FAIL_KEY: FAIL_KEY,
    FAIL_LIMIT: FAIL_LIMIT,
    normalized: normalized,
    parseConnectFailures: parseConnectFailures,
    nextConnectFailures: nextConnectFailures,
    decideStartup: decideStartup,
    decideProbeOutcome: decideProbeOutcome,
    probeMessage: probeMessage,
    connectHint: connectHint,
    buildConnectView: buildConnectView,
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
    var retryWrap = document.getElementById('retry');
    var retryText = document.getElementById('retryText');
    var retryBtn = document.getElementById('retryBtn');
    var retryChangeBtn = document.getElementById('retryChangeBtn');
    var needed = [
      form,
      urlInput,
      err,
      savedWrap,
      savedText,
      enterBtn,
      changeBtn,
      retryWrap,
      retryText,
      retryBtn,
      retryChangeBtn,
    ];
    for (var i = 0; i < needed.length; i += 1) {
      if (!needed[i]) return;
    }

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

    /** 读连续失败次数，存储不可用时按 0 次处理，最坏结果是晚几轮才重新询问。 */
    function readFailures() {
      try {
        return parseConnectFailures(localStorage.getItem(FAIL_KEY));
      } catch (e) {
        return 0;
      }
    }

    /** 写连续失败次数，隐私模式下写不进时本次判定仍按内存值推进，不阻塞用户。 */
    function writeFailures(count) {
      try {
        localStorage.setItem(FAIL_KEY, String(count));
      } catch (e) {
        // 存储不可用，计数不落盘，下次启动从 0 起算
      }
    }

    /** 读存档地址，读不到按未配置处理，走首装空表单。 */
    function readSaved() {
      try {
        return localStorage.getItem(KEY) || '';
      } catch (e) {
        return '';
      }
    }

    /** 把可见视图模型落到页内元素上，装配层唯一的写 DOM 处。 */
    function applyView(vm) {
      savedWrap.style.display = vm.savedVisible ? '' : 'none';
      savedText.textContent = vm.savedText;
      form.style.display = vm.formVisible ? '' : 'none';
      urlInput.value = vm.formValue;
      retryWrap.style.display = vm.retryVisible ? '' : 'none';
      retryText.textContent = vm.retryText;
      err.textContent = vm.errorText;
      if (vm.formVisible) urlInput.focus();
    }

    /** 存档卡片上的两颗按钮，入参为卡片上显示的地址，进入即跳转，更改即换空表单。 */
    function wireSavedCard(target) {
      enterBtn.onclick = function () {
        go(target);
      };
      changeBtn.onclick = function () {
        applyView(buildConnectView({ view: 'manual-empty', message: '' }));
      };
    }

    /**
     * 探测一个地址并按结论推进状态机，入参 target 为待验证地址，仅在有壳桥时调用
     * （无桥的桌面预览由 decideStartup 直接落到 saved-card，不会走到这里）。
     * 结论先算进连续失败计数再交给 decideProbeOutcome，各条出口都写回计数，
     * 成功进入时计数归零，下一轮启动因此不受本轮失败影响。
     */
    function probeAndRoute(target) {
      applyView(buildConnectView({ view: 'probing', message: '正在连接服务器…' }));
      var pending = probe(target);
      function settle(outcome) {
        var failures = nextConnectFailures(readFailures(), outcome);
        writeFailures(failures);
        var verdict = decideProbeOutcome({ outcome: outcome, failures: failures });
        if (verdict === 'navigate') {
          go(target);
          return;
        }
        if (verdict === 'retry') {
          applyView(buildConnectView({ view: 'retry', message: connectHint('retry', outcome) }));
          return;
        }
        applyView(
          buildConnectView({ view: 'manual-empty', message: connectHint('manual-empty', outcome) })
        );
      }
      pending
        .then(function (outcome) {
          settle(String(outcome));
        })
        .catch(function () {
          // 探测命令本身不可用（例如旧安装包里没有该命令），按不可达推进，不静默放行
          settle('unreachable');
        });
    }

    function submitHandler(event) {
      event.preventDefault();
      var ok = normalized(urlInput.value);
      var verdict = decideSubmit({ valid: !!ok, probeSupported: probeSupported });
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
          if (decideSubmit({ valid: true, probeSupported: true, outcome: String(outcome) }) === 'navigate') {
            finishSave(ok);
          } else {
            err.textContent =
              probeMessage(outcome) || '该地址连不上，请检查服务器状态或更换地址';
          }
        })
        .catch(function () {
          err.textContent = '该地址连不上，请检查服务器状态或更换地址';
        });
    }

    /** 探测放行后落库并跳转，存储不可用不阻塞进入，同时清掉连续失败计数。 */
    function finishSave(ok) {
      err.textContent = '';
      writeFailures(0);
      try {
        localStorage.setItem(KEY, ok);
      } catch (e) {
        // 隐私模式下存储不可用，本次进入仍然有效，仅下次启动需重填
      }
      go(ok);
    }

    function startup() {
      rememberSelf();
      var reconfigure = /[?&]reconfigure=1(&|$)/.test(window.location.search);
      var saved = readSaved();
      var view = api.decideStartup({
        reconfigureRequested: reconfigure,
        hasSaved: !!saved,
        probeSupported: probeSupported,
      });
      if (view === 'manual-empty') {
        applyView(
          buildConnectView({
            view: 'manual-empty',
            message: '上次没有成功进入系统，请确认服务器地址后重新填写',
          })
        );
        return;
      }
      if (view === 'first-install') {
        applyView(buildConnectView({ view: 'first-install', message: '' }));
        return;
      }
      if (view === 'saved-card') {
        applyView(buildConnectView({ view: 'saved-card', saved: saved }));
        wireSavedCard(saved);
        return;
      }
      retryBtn.onclick = function () {
        probeAndRoute(saved);
      };
      retryChangeBtn.onclick = function () {
        applyView(buildConnectView({ view: 'manual-empty', message: '' }));
      };
      probeAndRoute(saved);
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
