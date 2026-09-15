import { useEffect, useRef, useState } from 'react';
import { ShieldCheck } from 'lucide-react';
import { useShallow } from 'zustand/react/shallow';
import { useSettingsStore } from '@/stores/settingsStore';
import { androidMaterialTokens } from '@/lib/androidMaterial';
import { createAndroidLiquidRenderer, type LiquidRenderer } from '@/lib/androidLiquidRenderer';
import { useMediaPreference } from '@/hooks/useAndroidGlass';

export function AndroidLiquidArtwork() {
  const canvas = useRef<HTMLCanvasElement>(null);
  const renderer = useRef<LiquidRenderer | null>(null);
  const [available, setAvailable] = useState(false);
  const { theme, accent, reduceMotion } = useSettingsStore(
    useShallow((s) => ({
      theme: s.settings.theme,
      accent: s.settings.accentColor,
      reduceMotion: s.settings.reduceMotion,
    })),
  );
  const systemDark = useMediaPreference('(prefers-color-scheme: dark)');
  const systemReduced = useMediaPreference('(prefers-reduced-motion: reduce)');
  const dark = theme === 'dark' || (theme === 'system' && systemDark);
  const tokens = androidMaterialTokens(dark, accent);
  const appearance = {
    dark,
    reduced: reduceMotion || systemReduced,
    container: tokens['--md-primary-container'],
    accent: tokens['--accent-primary'],
    secondary: tokens['--md-secondary-container'],
    tertiary: tokens['--md-tertiary-container'],
  };
  const appearanceRef = useRef(appearance);
  appearanceRef.current = appearance;
  useEffect(() => {
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
  useEffect(
    () => renderer.current?.update(appearanceRef.current),
    [dark, accent, reduceMotion, systemReduced],
  );
  return (
    <div className="android-liquid-artwork" aria-hidden="true" data-liquid-ready={available}>
      <div className="android-liquid-fallback" />
      <canvas ref={canvas} />
      <ShieldCheck className="android-liquid-mark" />
    </div>
  );
}
