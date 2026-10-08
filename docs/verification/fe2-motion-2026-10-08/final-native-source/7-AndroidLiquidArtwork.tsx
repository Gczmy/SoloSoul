import { useLayoutEffect, useRef, useState, useSyncExternalStore } from 'react';
import { ShieldCheck } from 'lucide-react';
import { useShallow } from 'zustand/react/shallow';
import { useSettingsStore } from '@/stores/settingsStore';
import { getAndroidMaterialSnapshot, subscribeAndroidMaterial } from '@/lib/androidMaterial';
import { createAndroidLiquidRenderer, type LiquidRenderer } from '@/lib/androidLiquidRenderer';
import { useMediaPreference } from '@/hooks/useAndroidGlass';

export function AndroidLiquidArtwork() {
  const canvas = useRef<HTMLCanvasElement>(null);
  const renderer = useRef<LiquidRenderer | null>(null);
  const [available, setAvailable] = useState(false);
  const { reduceMotion } = useSettingsStore(
    useShallow((s) => ({
      reduceMotion: s.settings.reduceMotion,
    })),
  );
  const material = useSyncExternalStore(
    subscribeAndroidMaterial,
    getAndroidMaterialSnapshot,
    getAndroidMaterialSnapshot,
  );
  const { dark, colors: tokens } = material;
  const systemReduced = useMediaPreference('(prefers-reduced-motion: reduce)');
  const appearance = {
    dark,
    reduced: reduceMotion || systemReduced,
    container: tokens['--android-liquid-base'],
    accent: tokens['--accent-primary'],
    secondary: tokens['--md-secondary-container'],
    tertiary: tokens['--md-tertiary-container'],
  };
  const appearanceRef = useRef(appearance);
  appearanceRef.current = appearance;
  useLayoutEffect(() => {
    if (!canvas.current) return;
    renderer.current = createAndroidLiquidRenderer(
      canvas.current,
      appearanceRef.current,
      setAvailable,
    );
    return () => {
      renderer.current?.dispose();
      renderer.current = null;
    };
  }, []);
  useLayoutEffect(
    () => renderer.current?.update(appearanceRef.current),
    [material, reduceMotion, systemReduced],
  );
  return (
    <div className="android-liquid-artwork" aria-hidden="true" data-liquid-ready={available}>
      <div className="android-liquid-fallback" />
      <canvas ref={canvas} />
      <ShieldCheck className="android-liquid-mark" />
    </div>
  );
}
