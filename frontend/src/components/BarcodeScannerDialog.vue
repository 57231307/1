<script setup lang="ts">
import { ref, onBeforeUnmount, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElDialog, ElButton, ElAlert, ElSelect, ElOption } from 'element-plus';
import { Html5Qrcode, Html5QrcodeSupportedFormats, type CameraDevice } from 'html5-qrcode';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  scanned: [barcode: string];
}>();

const visible = ref(false);
const errorMessage = ref('');
const scanning = ref(false);
const readerId = 'barcode-scanner-region';

// 枚举到的视频输入设备与当前选中的 deviceId（桌面端多摄/单摄、手机前后置通用）
const cameras = ref<CameraDevice[]>([]);
const activeCameraId = ref('');

// 后置/环境摄像头标签关键字（不同浏览器 label 用词不一，按大小写不敏感匹配）
const BACK_CAMERA_RE = /(back|rear|environment|arri[èe]re|posto|trasero)/i;

let html5Qrcode: Html5Qrcode | null = null;

// 依据错误类型给出精确文案，禁止用通用 initError 吞掉"被拒/无设备/设备被占"的差异
function applyError(err: unknown) {
  scanning.value = false;
  const name = err instanceof DOMException ? err.name : '';
  if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
    errorMessage.value = t('barcodeScanner.camera.permissionDenied');
  } else if (name === 'NotFoundError' || name === 'DevicesNotFoundError') {
    errorMessage.value = t('barcodeScanner.camera.noCamera');
  } else if (
    name === 'OverconstrainedError' ||
    name === 'NotReadableError' ||
    name === 'TrackStartError' ||
    name === 'AbortError'
  ) {
    // 设备被占用 / 约束无法满足 / 轨道启动失败：可换设备或手动输入
    errorMessage.value = t('barcodeScanner.camera.cameraBusy');
  } else {
    errorMessage.value = t('barcodeScanner.camera.initError');
  }
  logger.error('[BarcodeScanner] camera init failed', err);
}

// 以指定 deviceId 启动一次扫码（deviceId 精确约束，桌面/手机通用）
async function startScanning(deviceId: string) {
  await stopScan();
  try {
    html5Qrcode = new Html5Qrcode(readerId, {
      formatsToSupport: [
        Html5QrcodeSupportedFormats.QR_CODE,
        Html5QrcodeSupportedFormats.EAN_13,
        Html5QrcodeSupportedFormats.EAN_8,
        Html5QrcodeSupportedFormats.CODE_128,
        Html5QrcodeSupportedFormats.CODE_39,
        Html5QrcodeSupportedFormats.UPC_A,
        Html5QrcodeSupportedFormats.UPC_E,
      ],
      verbose: false,
    });

    // start 传入 deviceId 字符串时，html5-qrcode 内部构造 { deviceId: { exact } }
    await html5Qrcode.start(
      deviceId,
      {
        fps: 10,
        qrbox: { width: 250, height: 250 },
      },
      decodedText => {
        logger.info(`[BarcodeScanner] recognized: ${decodedText}`);
        stopScan();
        emit('scanned', decodedText);
        close();
      },
      () => {
        // 每帧识别失败是正常现象（未对准），不做处理
      }
    );
    scanning.value = true;
    errorMessage.value = '';
    logger.info(`[BarcodeScanner] started camera deviceId=${deviceId}`);
  } catch (err: unknown) {
    applyError(err);
  }
}

async function initScanner() {
  errorMessage.value = '';
  scanning.value = false;
  cameras.value = [];
  activeCameraId.value = '';

  // getUserMedia 仅在安全上下文（HTTPS 或 localhost）可用：诚实前置判定，不再向下误试
  if (!window.isSecureContext) {
    errorMessage.value = t('barcodeScanner.camera.insecureContext');
    logger.warn('[BarcodeScanner] insecure context, camera API unavailable');
    return;
  }
  if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
    errorMessage.value = t('barcodeScanner.camera.notSupported');
    return;
  }

  try {
    // getCameras() 本身即触发权限弹窗（两端通用），随后枚举可用视频输入设备
    const devices = await Html5Qrcode.getCameras();
    cameras.value = devices;
    if (!devices.length) {
      errorMessage.value = t('barcodeScanner.camera.noCamera');
      logger.warn('[BarcodeScanner] no video input device found');
      return;
    }

    // 优先后置/环境摄像头，否则取第一个可用设备
    const backCamera = devices.find(d => BACK_CAMERA_RE.test(`${d.label} ${d.id}`));
    const chosen = backCamera ?? devices[0];
    activeCameraId.value = chosen.id;

    await nextTick();
    await startScanning(chosen.id);
  } catch (err: unknown) {
    applyError(err);
  }
}

async function onCameraChange(deviceId: string) {
  logger.info(`[BarcodeScanner] switching camera to deviceId=${deviceId}`);
  await startScanning(deviceId);
}

// 停止并清理当前实例的媒体流（切换/关闭/卸载复用，避免 stream 泄漏）
async function stopScan() {
  if (html5Qrcode) {
    try {
      if (html5Qrcode.isScanning) {
        await html5Qrcode.stop();
      }
    } catch {
      // 已经停止，忽略
    }
    try {
      html5Qrcode.clear();
    } catch {
      // DOM 不存在，忽略
    }
    html5Qrcode = null;
  }
  scanning.value = false;
}

function open() {
  visible.value = true;
  initScanner();
}

function close() {
  stopScan();
  visible.value = false;
}

function handleClosed() {
  stopScan();
}

onBeforeUnmount(() => {
  stopScan();
});

defineExpose({ open });
</script>

<template>
  <ElDialog
    v-model="visible"
    :title="t('barcodeScanner.camera.dialogTitle')"
    width="500px"
    :close-on-click-modal="false"
    @closed="handleClosed"
  >
    <ElAlert
      v-if="errorMessage"
      :title="errorMessage"
      type="warning"
      :closable="false"
      show-icon
      style="margin-bottom: 12px"
    />
    <div
      v-if="cameras.length > 1"
      style="margin-bottom: 12px; display: flex; align-items: center; gap: 8px"
    >
      <span style="white-space: nowrap">{{ t('barcodeScanner.camera.cameraSelect') }}</span>
      <ElSelect
        v-model="activeCameraId"
        :aria-label="t('barcodeScanner.camera.switchCamera')"
        style="flex: 1"
        @change="onCameraChange"
      >
        <ElOption
          v-for="(cam, i) in cameras"
          :key="cam.id"
          :value="cam.id"
          :label="cam.label || `${t('barcodeScanner.camera.cameraFallbackLabel')} ${i + 1}`"
        />
      </ElSelect>
    </div>
    <div :id="readerId" style="width: 100%; min-height: 300px"></div>
    <p v-if="scanning" style="text-align: center; color: #909399">
      {{ t('barcodeScanner.camera.scanning') }}
    </p>
    <template #footer>
      <ElButton @click="close">{{ t('barcodeScanner.camera.close') }}</ElButton>
    </template>
  </ElDialog>
</template>
