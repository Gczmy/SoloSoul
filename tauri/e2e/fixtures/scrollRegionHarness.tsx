import { useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { useActiveScrollRegion } from '../../src/hooks/useActiveScrollRegion';
import '../../src/styles/global.css';
import navigationStyles from '../../src/components/layout/SideNavigation.module.css';

/** 独立挂载生产hook，避免登录页的其他异步重绘掩盖滚动状态更新遗漏。 */
function Fixture() {
  useActiveScrollRegion();
  useEffect(() => {
    document.documentElement.dataset.scrollFixtureReady = 'true';
  }, []);
  return (
    <div style={{ position: 'fixed', inset: 0, display: 'flex', background: 'var(--bg-base)' }}>
      <aside id="scroll-sidebar" style={{ width: 150, flexShrink: 0, overflow: 'auto' }}>
        <button id="side-focus">Sidebar</button>
        <section
          className={navigationStyles.sideNav}
          style={
            {
              width: 140,
              height: 300,
              minWidth: 0,
              '--tools-available-height': '300px',
            } as React.CSSProperties
          }
        >
          <div
            id="tools-scroll"
            tabIndex={-1}
            className={navigationStyles.foldableContent}
            style={{ width: 130 }}
          >
            <div style={{ height: 1000 }}>Tools content</div>
          </div>
        </section>
        <div style={{ height: 1800 }}>Side content</div>
      </aside>
      <main id="scroll-main" style={{ flex: 1, minWidth: 0, overflow: 'auto' }}>
        <button id="main-focus">Main</button>
        <div
          id="horizontal"
          style={{ width: 240, height: 70, overflowX: 'auto', overflowY: 'hidden', margin: 20 }}
        >
          <div style={{ width: 800, height: 50 }}>Horizontal content</div>
        </div>
        <textarea
          id="text-scroll"
          defaultValue={'Long line\n'.repeat(40)}
          style={{ width: 240, height: 90, display: 'block', margin: 20 }}
        />
        <div style={{ height: 1800 }}>Main content</div>
      </main>
    </div>
  );
}
createRoot(document.getElementById('fixture-root')!).render(<Fixture />);
