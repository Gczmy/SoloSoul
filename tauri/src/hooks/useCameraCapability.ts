import { useEffect, useState } from 'react';
import {
  getCameraCapability,
  preloadCameraCapability,
  type CameraCapability,
} from '@/lib/cameraCapability';

/**
 * 订阅设备摄像头能力。首次调用触发预加载（模块级缓存，全局只检测一次），
 * 检测完成后组件自动重渲染并返回最终结果。
 */
export function useCameraCapability(): CameraCapability {
  const [capability, setCapability] = useState<CameraCapability>(getCameraCapability);

  useEffect(() => {
    let mounted = true;
    preloadCameraCapability()
      .then((cap) => {
        if (mounted) setCapability(cap);
      })
      .catch(() => {
        // 检测失败时保持 'unknown'，由调用方保守处理
      });
    return () => {
      mounted = false;
    };
  }, []);

  return capability;
}
