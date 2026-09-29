<script setup lang="ts">
import { ref, onBeforeUnmount, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElDialog, ElButton, ElAlert } from 'element-plus';
import { Html5Qrcode, Html5QrcodeSupportedFormats } from 'html5-qrcode';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  scanned: [barcode: string];
}>();

const visible = ref(false);
const errorMessage = ref('');
const scanning = ref(false);
const readerId = 'barcode-scanner-region';

let html5Qrcode: Html5Qrcode | null = null;

async function startCamera() {
  errorMessage.value = '';
  scanning.value = false;

  if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
    errorMessage.value = t('barcodeScanner.camera.notSupported');
    return;
  }

  await nextTick();

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

    scanning.value = true;
    await html5Qrcode.start(
      { facingMode: 'environment' },
      {
        fps: 10,
        qrbox: { width: 250, height: 250 },
      },
      decodedText => {
        logger.info(`[BarcodeScanner] recognized: ${decodedText}`);
        stopCamera();
        emit('scanned', decodedText);
        close();
      },
      () => {
        // 每帧识别失败是正常现象（未对准），不做处理
      }
    );
  } catch (err: unknown) {
    scanning.value = false;
    const name = err instanceof DOMException ? err.name : '';
    if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
      errorMessage.value = t('barcodeScanner.camera.permissionDenied');
    } else if (name === 'NotFoundError') {
      errorMessage.value = t('barcodeScanner.camera.noCamera');
    } else {
      errorMessage.value = t('barcodeScanner.camera.initError');
    }
    logger.error('[BarcodeScanner] camera init failed', err);
  }
}

function stopCamera() {
  if (html5Qrcode) {
    try {
      if (html5Qrcode.isScanning) {
        html5Qrcode.stop().catch(() => {
          // stop 时若已停止则忽略
        });
      }
      html5Qrcode.clear();
    } catch {
      // 已经停止或 DOM 不存在，忽略
    }
    html5Qrcode = null;
  }
  scanning.value = false;
}

function open() {
  visible.value = true;
  startCamera();
}

function close() {
  stopCamera();
  visible.value = false;
}

onBeforeUnmount(() => {
  stopCamera();
});

defineExpose({ open });
</script>

<template>
  <ElDialog
    v-model="visible"
    :title="t('barcodeScanner.camera.dialogTitle')"
    width="500px"
    :close-on-click-modal="false"
    @closed="stopCamera"
  >
    <ElAlert
      v-if="errorMessage"
      :title="errorMessage"
      type="warning"
      :closable="false"
      show-icon
      style="margin-bottom: 12px"
    />
    <div :id="readerId" style="width: 100%; min-height: 300px"></div>
    <p v-if="scanning" style="text-align: center; color: #909399">
      {{ t('barcodeScanner.camera.scanning') }}
    </p>
    <template #footer>
      <ElButton @click="close">{{ t('barcodeScanner.camera.close') }}</ElButton>
    </template>
  </ElDialog>
</template>
