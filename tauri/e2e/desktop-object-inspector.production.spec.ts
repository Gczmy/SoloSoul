import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

const LONG_OBJECT_NAME =
  'A record with a very long descriptive object name that must scroll to reveal the ending';
const LONG_HISTORY_NAME = 'A saved version with a very long descriptive object name';
const LONG_FILE_NAME = 'A document with a very long descriptive file name.txt';

async function setup(
  page: Page,
  platform: 'windows' | 'macos',
  theme: 'light' | 'dark',
  longNames = false,
) {
  const firstName = longNames ? LONG_OBJECT_NAME : 'Passport for navigation';
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__MOCK_PLATFORM__ = '${platform}';
    localStorage.setItem('i18nextLng','en-US');
    const prefs={theme:'${theme}',accentColor:'ocean',hasSeenOnboarding:true,defaultLightTheme:'warm-stone',defaultDarkTheme:'warm-stone-dark'};
    localStorage.setItem('solosoul_ui_prefs',JSON.stringify(prefs));
    const objects=['one','two','empty'].map((id,i)=>({id,name:i===0?${JSON.stringify(firstName)}:i===1?'Second record':'Empty record',
      typeId:'identity',sectionType:'identity',sensitivityLevel:'public',createdAt:'2026-10-20',updatedAt:'2026-10-20',
      isDeleted:false,templateId:null,templateType:null,tags:[],hasAttachments:false,
      properties:{__fields:Object.fromEntries(Array.from({length:14},(_,n)=>['field'+n,{name:'Public field '+n,type:'text',sensitivityLevel:'public'}])),
        ...Object.fromEntries(Array.from({length:14},(_,n)=>['field'+n,'Editor body '+id+' '+n]))},
      propertyLabels:Object.fromEntries(Array.from({length:14},(_,n)=>['field'+n,'public']))}));
    window.__INSPECTOR_CALLS__=[];
    let frame=false;
    window.__E2E_MOCKS__={
      ui_get_preferences:()=>prefs,
      vault_check_directory:()=>true,
      ocr_get_model_status:()=>({installed:true,bundled:true}),
      sync_list_conflicts:()=>[],
      object_list:({filter}={})=>filter?.typeId==='page'?[]:objects,
      object_get:async({objectId,accountId})=>{
        if(objectId==='two') await new Promise(resolve=>setTimeout(resolve,300));
        return {...objects.find(o=>o.id===objectId),accountId};
      },
      snapshot_list:({objectId})=>{
        window.__INSPECTOR_CALLS__.push(['list',objectId]);
        return objectId==='empty'?[]:Array.from({length:32},(_,i)=>({id:objectId+'-snap-'+i,
          timestamp:Date.UTC(2026,9,20,12,i),triggeredBy:'user_edit',diffSummary:'diff_updated'}));
      },
      snapshot_get_data:({snapshotId})=>{
        window.__INSPECTOR_CALLS__.push(['body',snapshotId]);
        return {name:${longNames ? JSON.stringify(LONG_HISTORY_NAME) : "'Snapshot '+snapshotId"},properties:{public:'PUBLIC '+snapshotId,secret:'PRIVATE '+snapshotId},propertyLabels:{public:'public',secret:'sensitive'}};
      },
      attachment_list:({objectId,showDeleted})=>{
        window.__INSPECTOR_CALLS__.push(['attachments',objectId,showDeleted]);
        return objectId==='empty'?[]:Array.from({length:36},(_,i)=>({id:objectId+'-file-'+i,objectId,
          fileName:${longNames ? `i===35?${JSON.stringify(LONG_FILE_NAME)}:i===34?'short.txt':objectId+' attachment '+i` : "objectId+' attachment '+i+' with a long descriptive name for the supporting document.pdf'"},mimeType:'application/pdf',
          sizeBytes:1024*(i+1),createdAt:new Date(Date.UTC(2026,9,20,12,i)).toISOString()}));
      },
      set_titlebar_color:()=>({platform:'${platform}',material:'${platform === 'macos' ? 'liquid-glass' : 'mica'}',reduceMotion:false,highContrast:false}),
      set_titlebar_controls:args=>{if(args.windowsFrame!==undefined) frame=args.windowsFrame;},
      get_window_layout:()=>({platform:'${platform}',titlebarHeight:52,trafficLightsRight:${platform === 'macos' ? 80 : 0},
        windowsFrame:frame?{enabled:true,headerHeight:52,dpi:96,maximized:false,captionBounds:[window.innerWidth-146,0,146,30]}:undefined})
    };
    const invoke=window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke=async(cmd,args)=>{
      const result=await invoke(cmd,args);
      return cmd==='ui_get_preferences'||cmd==='user_data_get_preferences'?{...result,...prefs}:result;
    };
  `,
  });
  await login(page);
  await page
    .locator('#desktop-page-pane')
    .getByRole('button', { name: firstName, exact: true })
    .click();
  await expect(page).toHaveURL(/\/editor\/one$/);
  await expect(page.getByLabel('Object Name', { exact: true })).toHaveValue(firstName);
}

async function expectReselectionToPreserveInspector(page: Page) {
  const panel = page.locator('[data-object-inspector]');
  const panelNode = await panel.elementHandle();
  const listNode = await panel.locator('[data-object-inspector-scroll]').elementHandle();
  const scrollTop = await listNode!.evaluate((element) => element.scrollTop);
  const calls = await page.evaluate('JSON.stringify(window.__INSPECTOR_CALLS__)');
  for (let click = 0; click < 3; click++) {
    await page.locator('#desktop-page-pane [data-object-id="one"]').click();
  }
  expect(await panelNode!.evaluate((element) => element.isConnected)).toBe(true);
  expect(await listNode!.evaluate((element) => element.isConnected)).toBe(true);
  expect(await listNode!.evaluate((element) => element.scrollTop)).toBe(scrollTop);
  expect(await page.evaluate('JSON.stringify(window.__INSPECTOR_CALLS__)')).toBe(calls);
  await expect(panel.getByRole('status')).toHaveCount(0);
}

for (const platform of ['windows', 'macos'] as const)
  for (const theme of ['light', 'dark'] as const) {
    test(`${platform} ${theme}: docked chronological tabs, independent scroll and synchronized objects`, async ({
      page,
    }, info) => {
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await setup(page, platform, theme);
      await expect(page.locator('script[type="module"]').first()).toHaveAttribute(
        'src',
        /^\/assets\/.+\.js$/,
      );
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
      const toggle = page.locator('[data-object-inspector-toggle]');
      await expect(toggle).toHaveAttribute('aria-expanded', 'false');
      const leftToggle = page.locator('[data-appbar] [aria-controls="desktop-page-pane"]');
      await leftToggle.hover();
      const leftHint = page.locator('[data-page-pane-toggle-tooltip]');
      await expect(leftHint).toHaveText('Show/hide sidebar');
      const hintStyle = (element: Element) => {
        const style = getComputedStyle(element);
        return [
          'background-color',
          'color',
          'border-radius',
          'box-shadow',
          'font-size',
          'animation-name',
        ].map((name) => style.getPropertyValue(name));
      };
      const expectedHintStyle = await leftHint.evaluate(hintStyle);
      await toggle.hover();
      await expect(leftHint).toHaveCount(0);
      const rightHint = page.locator('[data-object-inspector-toggle-tooltip]');
      await expect(rightHint).toHaveText('Show/hide object details');
      expect(await rightHint.evaluate(hintStyle)).toEqual(expectedHintStyle);
      await expect(toggle).not.toHaveAttribute('title');
      await expect(toggle).toHaveAttribute(
        'aria-describedby',
        (await rightHint.getAttribute('id'))!,
      );
      const hintBox = (await rightHint.boundingBox())!;
      expect(hintBox.x).toBeGreaterThanOrEqual(8);
      expect(hintBox.x + hintBox.width).toBeLessThanOrEqual(1272);
      await page.keyboard.press('Escape');
      await expect(rightHint).toHaveCount(0);
      await page.mouse.move(640, 400);
      await toggle.focus();
      await expect(rightHint).toBeVisible();
      await page.keyboard.press('Tab');
      await expect(rightHint).toHaveCount(0);
      const toolbar = page.locator('[data-editor-content-toolbar]');
      const header = page.locator('[data-appbar]');
      const actionArea = platform === 'windows' ? toolbar : header;
      await expect(actionArea.getByRole('button', { name: 'Cancel', exact: true })).toBeVisible();
      await expect(actionArea.getByRole('button', { name: 'Save', exact: true })).toBeVisible();
      await expect(actionArea.locator('[data-object-inspector-toggle]')).toHaveCount(1);
      if (platform === 'windows') {
        await expect(header.getByRole('button', { name: 'Save', exact: true })).toHaveCount(0);
        await expect(toolbar).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
      } else {
        await expect(toolbar).toHaveCount(0);
        await expect(page.locator('[data-shell-content]')).toHaveCSS('mask-image', 'none');
      }
      await toggle.click();
      const panel = page.locator('[data-object-inspector]');
      await expect(panel).toBeVisible();
      await expect(panel.locator('[data-history-snapshot]').first()).toHaveAttribute(
        'data-history-snapshot',
        'one-snap-31',
      );
      expect(
        await page.evaluate('window.__INSPECTOR_CALLS__.filter(c=>c[0]==="body").length'),
      ).toBe(0);
      const content = page.locator('[data-shell-content]');
      const body = await content.boundingBox();
      const dock = await panel.boundingBox();
      expect(Math.abs(body!.x + body!.width - dock!.x)).toBeLessThan(2);
      expect(dock!.width).toBe(340);
      expect((await page.locator('[data-appbar]').boundingBox())!.height).toBe(52);
      await expect(page.locator('#desktop-navigation')).toHaveCSS('width', '56px');
      if (platform === 'windows') {
        const bar = (await toolbar.boundingBox())!;
        expect(bar.height).toBe(52);
        expect(Math.abs(bar.y + bar.height - body!.y)).toBeLessThan(1);
        expect(Math.abs(bar.y + bar.height - dock!.y)).toBeLessThan(1);
        expect(Math.abs(bar.x + bar.width - dock!.x - dock!.width)).toBeLessThan(1);
        const buttons = await toolbar.locator('button').evaluateAll((els) =>
          els.map((el) => {
            const r = el.getBoundingClientRect();
            return { x: r.x, y: r.y, height: r.height, right: r.right };
          }),
        );
        expect(buttons).toHaveLength(3);
        const form = page.locator('[data-object-editor-form]');
        const formBox = (await form.boundingBox())!;
        expect(Math.abs(buttons[1].right - formBox.x - formBox.width)).toBeLessThan(1);
        expect(buttons[0].x).toBeLessThan(buttons[1].x);
        expect(buttons[1].x).toBeLessThan(buttons[2].x);
        expect(buttons.every((button) => button.height === 32 && button.y === buttons[0].y)).toBe(
          true,
        );
        await expect(content).not.toHaveCSS('mask-image', 'none');
        const name = page.getByLabel('Object Name', { exact: true });
        await name.fill('UNSAVED WINDOWS DRAFT');
        const contentHandle = await content.elementHandle();
        const field = content.getByText('Public field 5', { exact: true });
        await content.evaluate((el) => {
          el.scrollTop = 600;
        });
        await expect.poll(() => content.evaluate((el) => el.scrollTop)).toBe(600);
        expect((await toolbar.boundingBox())!).toEqual(bar);
        await expect(toolbar.getByRole('button', { name: 'Save', exact: true })).toBeInViewport();
        await field.click();
        await page.screenshot({ path: info.outputPath(`windows-${theme}-toolbar-scroll.png`) });
        expect(await contentHandle!.evaluate((el) => el.isConnected)).toBe(true);
        await expect(name).toHaveValue('UNSAVED WINDOWS DRAFT');
        await content.evaluate((el) => {
          el.scrollTop = 0;
        });
        await name.fill('Passport for navigation');
      }
      await panel.locator('[data-history-snapshot]').first().click();
      await expect(panel.getByText('PUBLIC one-snap-31', { exact: true })).toBeVisible();
      await expect(panel.getByText('PRIVATE one-snap-31', { exact: true })).toHaveCount(0);
      await panel.getByRole('button', { name: 'Click to reveal', exact: true }).click();
      await expect(panel.getByText('PRIVATE one-snap-31', { exact: true })).toBeVisible();
      const scroller = panel.locator('[data-object-inspector-scroll]');
      const panelHeader = await panel.locator('header').boundingBox();
      const editorScroll = await content.evaluate((el) => el.scrollTop);
      await scroller.evaluate((el) => (el.scrollTop = el.scrollHeight));
      expect(await content.evaluate((el) => el.scrollTop)).toBe(editorScroll);
      expect(await panel.locator('header').boundingBox()).toEqual(panelHeader);
      await panel.getByRole('button', { name: 'Load more', exact: true }).click();
      await expect(panel.locator('[data-history-snapshot]')).toHaveCount(32);
      await expectReselectionToPreserveInspector(page);
      await expect(panel.locator('[data-history-snapshot]').first()).toHaveAttribute(
        'aria-expanded',
        'true',
      );
      await panel.getByRole('tab', { name: 'Attachments', exact: true }).click();
      await expect(panel.locator('[data-inspector-attachment]').first()).toHaveAttribute(
        'data-inspector-attachment',
        'one-file-35',
      );
      await expect(panel.getByText('PRIVATE one-snap-31', { exact: true })).toHaveCount(0);
      const tabs = await panel.getByRole('tablist').boundingBox();
      await scroller.evaluate((el) => (el.scrollTop = el.scrollHeight));
      expect(await panel.getByRole('tablist').boundingBox()).toEqual(tabs);
      await panel.getByRole('button', { name: 'Load more', exact: true }).click();
      await expect(panel.locator('[data-inspector-attachment]')).toHaveCount(36);
      await expectReselectionToPreserveInspector(page);
      await panel.getByRole('tab', { name: 'History', exact: true }).click();
      await expect(panel.locator('[data-history-snapshot]').first()).toHaveAttribute(
        'aria-expanded',
        'false',
      );
      const mainHandle = await content.elementHandle();
      await page
        .locator('#desktop-page-pane')
        .getByRole('button', { name: 'Second record', exact: true })
        .click();
      await expect(panel).toHaveAttribute('data-object-id', 'two');
      await expect(panel.locator('[data-history-snapshot]').first()).toHaveAttribute(
        'data-history-snapshot',
        'two-snap-31',
      );
      await expect(panel.getByText('PRIVATE one-snap-31', { exact: true })).toHaveCount(0);
      expect(await mainHandle!.evaluate((el) => el.isConnected)).toBe(true);
      await page.screenshot({ path: info.outputPath(`${platform}-${theme}-history.png`) });
      await page
        .locator('#desktop-page-pane')
        .getByRole('button', { name: 'Empty record', exact: true })
        .click();
      await expect(panel.getByText('No history snapshots yet.', { exact: true })).toBeVisible();
      await panel.getByRole('tab', { name: 'Attachments', exact: true }).click();
      await expect(panel.getByText('No attachments', { exact: true })).toBeVisible();
      await panel.getByRole('button', { name: 'Close object details' }).click();
      await expect(toggle).toHaveAttribute('aria-expanded', 'false');
      await expect(toggle).toBeFocused();
      expect(errors).toEqual([]);
    });
  }

for (const platform of ['windows', 'macos'] as const)
  for (const theme of ['light', 'dark'] as const)
    for (const tab of ['history', 'attachments'] as const) {
      test(`${platform} ${theme}: atomic ${tab} handoff never paints a blank or misattributed list`, async ({
        page,
      }, info) => {
        await setup(page, platform, theme);
        await page.locator('[data-object-inspector-toggle]').click();
        const panel = page.locator('[data-object-inspector]');
        await panel
          .getByRole('tab', { name: tab === 'history' ? 'History' : 'Attachments', exact: true })
          .click();
        const attribute = tab === 'history' ? 'data-history-snapshot' : 'data-inspector-attachment';
        const rows = panel.locator(`[${attribute}]`);
        await expect(rows.first()).toHaveAttribute(
          attribute,
          tab === 'history' ? 'one-snap-31' : 'one-file-35',
        );
        const panelHandle = await panel.elementHandle();
        const listHandle = await panel.locator('[data-object-inspector-scroll]').elementHandle();
        const samples: unknown[] = [];
        for (const order of ['editor-first', 'metadata-first'] as const) {
          if (tab === 'history') {
            await rows.first().click();
            await panel.getByRole('button', { name: 'Click to reveal', exact: true }).click();
            await expect(panel.getByText('PRIVATE one-snap-31', { exact: true })).toBeVisible();
          }
          await page.evaluate(`(() => {
            const mocks = window.__E2E_MOCKS__;
            const objectGet = mocks.object_get;
            const command = ${JSON.stringify(tab === 'history' ? 'snapshot_list' : 'attachment_list')};
            const list = mocks[command];
            const gate = window.__INSPECTOR_HANDOFF__ = {started: [], frames: [], running: true, metadataDone: false};
            const editor = new Promise(done => {gate.editor = done;});
            const metadata = new Promise(done => {gate.metadata = done;});
            mocks.object_get = async args => {
              if(args.objectId === 'two') {gate.started.push('editor'); await editor;}
              return objectGet(args);
            };
            mocks[command] = async args => {
              if(args.objectId === 'two') {gate.started.push('metadata'); await metadata;}
              const result = await list(args);
              if(args.objectId === 'two') gate.metadataDone = true;
              return result;
            };
            const sample = () => {
              const panel = document.querySelector('[data-object-inspector]');
              const list = panel.querySelector('[data-object-inspector-scroll]');
              gate.frames.push({id: panel.dataset.objectId,
                name: panel.querySelector('header [data-hover-scroll-label]').textContent,
                first: list.querySelector('[${attribute}]')?.getAttribute('${attribute}'),
                count: list.querySelectorAll('[${attribute}]').length,
                loading: !!list.querySelector('[role="status"]')});
              if(gate.running) requestAnimationFrame(sample);
            };
            requestAnimationFrame(sample);
          })()`);
          await page.locator('#desktop-page-pane [data-object-id="two"]').click();
          await expect
            .poll(() => page.evaluate('window.__INSPECTOR_HANDOFF__.started.slice().sort()'))
            .toEqual(['editor', 'metadata']);
          await expect(panel).toHaveAttribute('data-inspector-target-id', 'two');
          await expect(panel).toHaveAttribute('data-object-id', 'one');
          await expect(panel.locator('[data-inspector-content]')).toHaveAttribute('inert', '');
          await expect(panel.getByText('PRIVATE one-snap-31', { exact: true })).toHaveCount(0);
          await page.evaluate(
            `window.__INSPECTOR_HANDOFF__.${order === 'editor-first' ? 'editor' : 'metadata'}()`,
          );
          if (order === 'editor-first') {
            await expect(page.getByLabel('Object Name', { exact: true })).toHaveValue(
              'Second record',
            );
          } else {
            await expect
              .poll(() => page.evaluate('window.__INSPECTOR_HANDOFF__.metadataDone'))
              .toBe(true);
          }
          await expect(panel).toHaveAttribute('data-object-id', 'one');
          await expect(panel.locator('header')).toContainText('Passport for navigation');
          await expect
            .poll(() => page.evaluate('window.__INSPECTOR_HANDOFF__.frames.length'))
            .toBeGreaterThan(8);
          await page.evaluate(
            `window.__INSPECTOR_HANDOFF__.${order === 'editor-first' ? 'metadata' : 'editor'}()`,
          );
          await expect(panel).toHaveAttribute('data-object-id', 'two');
          await expect(panel.locator('header')).toContainText('Second record');
          await expect(rows.first()).toHaveAttribute(
            attribute,
            tab === 'history' ? 'two-snap-31' : 'two-file-35',
          );
          await expect(panel.locator('[data-inspector-content]')).not.toHaveAttribute('inert', '');
          await expect
            .poll(() => page.evaluate('window.__INSPECTOR_HANDOFF__.frames.at(-1)?.id'))
            .toBe('two');
          const frames = await page.evaluate(`(() => {
            const gate = window.__INSPECTOR_HANDOFF__;
            gate.running = false;
            return gate.frames;
          })()`);
          expect(frames.length).toBeGreaterThan(8);
          for (const frame of frames) {
            expect(frame.count).toBeGreaterThan(0);
            expect(frame.loading).toBe(false);
            expect(frame.first).toMatch(new RegExp(`^${frame.id}-`));
            expect(frame.name).toBe(
              frame.id === 'one' ? 'Passport for navigation' : 'Second record',
            );
          }
          samples.push({ order, frames });
          expect(await panelHandle!.evaluate((element) => element.isConnected)).toBe(true);
          expect(await listHandle!.evaluate((element) => element.isConnected)).toBe(true);
          expect(
            await page.evaluate(
              `window.__INSPECTOR_CALLS__.filter(c=>c[0]===${JSON.stringify(tab === 'history' ? 'list' : 'attachments')}&&c[1]==='two').length`,
            ),
          ).toBe(order === 'editor-first' ? 1 : 2);
          if (order === 'editor-first') {
            await page.locator('#desktop-page-pane [data-object-id="one"]').click();
            await expect(panel).toHaveAttribute('data-object-id', 'one');
            await expect(rows.first()).toHaveAttribute(
              attribute,
              tab === 'history' ? 'one-snap-31' : 'one-file-35',
            );
          }
        }
        await info.attach('object-handoff-frames', {
          body: JSON.stringify(samples),
          contentType: 'application/json',
        });
      });
    }

test('narrow desktop temporarily hides the left pane, preserves drafts and restores preference', async ({
  page,
}) => {
  await setup(page, 'windows', 'light');
  await page.setViewportSize({ width: 800, height: 700 });
  const name = page.getByLabel('Object Name', { exact: true });
  await name.fill('UNSAVED DRAFT');
  const toggle = page.locator('[data-object-inspector-toggle]');
  await toggle.click();
  await expect(page.locator('[data-object-inspector]')).toBeVisible();
  await expect(page.locator('#desktop-page-pane')).toHaveAttribute('inert', '');
  await expect(page.locator('[data-shell-content]')).toHaveJSProperty(
    'scrollWidth',
    await page.locator('[data-shell-content]').evaluate((el) => el.clientWidth),
  );
  await page.getByRole('button', { name: 'Close object details' }).click();
  await expect(page.locator('#desktop-page-pane')).not.toHaveAttribute('inert', '');
  await expect(name).toHaveValue('UNSAVED DRAFT');
  expect(await page.evaluate("localStorage.getItem('solosoul_page_pane_open')")).not.toBe('false');
  await toggle.click();
  await page.locator('[data-appbar] button[aria-controls="desktop-page-pane"]').click();
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  await expect(page.locator('#desktop-page-pane')).not.toHaveAttribute('inert', '');
  await expect(name).toHaveValue('UNSAVED DRAFT');
});

for (const theme of ['light', 'dark'] as const) {
  test(`windows ${theme}: save tracks the form edge throughout inspector motion and rapid reversal`, async ({
    page,
  }, info) => {
    await setup(page, 'windows', theme);
    const name = page.getByLabel('Object Name', { exact: true });
    await name.fill('UNSAVED ALIGNMENT DRAFT');
    const main = await page.locator('[data-shell-content]').elementHandle();
    const save = await page.getByRole('button', { name: 'Save', exact: true }).elementHandle();
    for (const width of [1280, 1600, 800]) {
      await page.setViewportSize({ width, height: 800 });
      await expect(page.locator('[data-pane-animating], [data-inspector-animating]')).toHaveCount(
        0,
      );
      for (const rapid of [false, true]) {
        const frames = await page.evaluate(async (rapid) => {
          const toggle = document.querySelector<HTMLButtonElement>(
            '[data-object-inspector-toggle]',
          )!;
          const frameSamples: Array<{
            formRight: number;
            saveRight: number;
            saveLeft: number;
            cancelRight: number;
            saveY: number;
            cancelY: number;
            toggleLeft: number;
            toggleRight: number;
          }> = [];
          const start = performance.now();
          let reversed = false;
          toggle.click();
          await new Promise<void>((resolve) => {
            const sample = () => {
              const form = document
                .querySelector('[data-object-editor-form]')!
                .getBoundingClientRect();
              const actions = document.querySelectorAll('[data-editor-form-actions] button');
              const cancel = actions[0].getBoundingClientRect();
              const save = actions[1].getBoundingClientRect();
              const control = toggle.getBoundingClientRect();
              frameSamples.push({
                formRight: form.right,
                saveRight: save.right,
                saveLeft: save.left,
                cancelRight: cancel.right,
                saveY: save.y,
                cancelY: cancel.y,
                toggleLeft: control.left,
                toggleRight: control.right,
              });
              if (rapid && !reversed && performance.now() - start > 100) {
                toggle.click();
                reversed = true;
              }
              if (performance.now() - start < 550) requestAnimationFrame(sample);
              else resolve();
            };
            requestAnimationFrame(sample);
          });
          return frameSamples;
        }, rapid);
        expect(frames.length).toBeGreaterThan(4);
        const badFrames = frames.filter(
          (frame) =>
            Math.abs(frame.formRight - frame.saveRight) > 1 ||
            frame.cancelRight >= frame.saveLeft ||
            frame.saveRight + 7 > frame.toggleLeft ||
            frame.cancelY !== frame.saveY ||
            Math.abs(frame.toggleRight - frames[0].toggleRight) > 1,
        );
        expect(badFrames).toEqual([]);
        if (!rapid) {
          expect(
            Math.max(...frames.map((frame) => frame.saveRight)) -
              Math.min(...frames.map((frame) => frame.saveRight)),
          ).toBeGreaterThan(40);
        }
        await expect(name).toHaveValue('UNSAVED ALIGNMENT DRAFT');
        expect(await main!.evaluate((el) => el.isConnected)).toBe(true);
        expect(await save!.evaluate((el) => el.isConnected)).toBe(true);
      }
    }
    await page.screenshot({ path: info.outputPath(`windows-${theme}-aligned-actions.png`) });
  });
}

for (const platform of ['windows', 'macos'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${platform} ${theme}: sidebar previews first appear complete without motion or resizing`, async ({
      page,
    }) => {
      await setup(page, platform, theme);
      await page.setViewportSize({ width: 1280, height: 480 });
      const pane = page.locator('#desktop-page-pane');
      const card = page.locator('[data-page-pane-preview]');
      const startSampling = async () =>
        page.evaluate(() => {
          const frames: Array<{
            x: number;
            y: number;
            width: number;
            height: number;
            opacity: string;
            transform: string;
            animations: number;
            loading: boolean;
          }> = [];
          let frame = 0;
          const sample = () => {
            const card = document.querySelector<HTMLElement>('[data-page-pane-preview]');
            if (card) {
              const style = getComputedStyle(card);
              if (style.visibility !== 'hidden' && style.display !== 'none') {
                const rect = card.getBoundingClientRect();
                frames.push({
                  x: rect.x,
                  y: rect.y,
                  width: rect.width,
                  height: rect.height,
                  opacity: style.opacity,
                  transform: style.transform,
                  animations: card.getAnimations().length,
                  loading: Boolean(card.querySelector('[role="status"]')),
                });
              }
            }
            frame = requestAnimationFrame(sample);
          };
          sample();
          Object.assign(window, {
            __stopPreviewSamples: () => {
              cancelAnimationFrame(frame);
              return frames;
            },
          });
        });
      const expectStable = async () => {
        await page.waitForTimeout(200);
        const frames = (await page.evaluate('window.__stopPreviewSamples()')) as Array<{
          x: number;
          y: number;
          width: number;
          height: number;
          opacity: string;
          transform: string;
          animations: number;
          loading: boolean;
        }>;
        expect(frames.length).toBeGreaterThan(4);
        expect(
          frames.filter(
            (frame) =>
              frame.opacity !== '1' ||
              frame.transform !== 'none' ||
              frame.animations > 0 ||
              frame.loading ||
              Math.abs(frame.x - frames[0].x) > 0.5 ||
              Math.abs(frame.y - frames[0].y) > 0.5 ||
              Math.abs(frame.width - frames[0].width) > 0.5 ||
              Math.abs(frame.height - frames[0].height) > 0.5,
          ),
        ).toEqual([]);
      };
      await startSampling();
      await pane.locator('[data-page-id="identity"]').hover();
      await expect(card.getByRole('heading', { name: 'Identity', exact: true })).toBeVisible();
      await expectStable();
      await page.keyboard.press('Escape');
      await expect(card).toHaveCount(0);
      await page.evaluate(() => {
        const state = window as unknown as {
          __E2E_MOCKS__: Record<string, (args: unknown) => Promise<unknown>>;
          __finishPreview?: () => Promise<void>;
        };
        const original = state.__E2E_MOCKS__.object_get;
        state.__E2E_MOCKS__.object_get = (args) =>
          new Promise((resolve) => {
            state.__finishPreview = async () => resolve(await original(args));
          });
      });
      await startSampling();
      await pane.locator('[data-object-id="two"]').hover();
      await expect(card).toHaveCount(1);
      await expect(card).toBeHidden();
      await page.waitForTimeout(250);
      await expect(card).toBeHidden();
      await page.evaluate('window.__finishPreview()');
      await expect(card.getByRole('heading', { name: 'Second record', exact: true })).toBeVisible();
      await expectStable();
      const bounds = (await card.boundingBox())!;
      expect(bounds.y).toBeGreaterThanOrEqual(52);
      expect(bounds.y + bounds.height).toBeLessThanOrEqual(480);
      await card.getByRole('button', { name: 'Edit object', exact: true }).click();
      await expect(page).toHaveURL(/\/editor\/two$/);
    });
  }
}

for (const platform of ['windows', 'macos'] as const) {
  for (const theme of ['light', 'dark'] as const) {
    test(`${platform} ${theme}: inspector long names scroll without moving rows or loading extra history`, async ({
      page,
    }) => {
      await setup(page, platform, theme, true);
      await page.locator('[data-object-inspector-toggle]').click();
      const panel = page.locator('[data-object-inspector]');
      const objectName = panel.locator('header [data-hover-scroll-label]');
      await expect(objectName).toHaveAttribute('data-overflow', 'true');
      const headerBounds = await panel.locator('header').boundingBox();
      const textTrack = (label: ReturnType<Page['locator']>) =>
        label.locator('[data-hover-scroll-track]');
      const finishTravel = async (label: ReturnType<Page['locator']>) => {
        await expect(label).toHaveAttribute('data-scroll-state', 'forward');
        await textTrack(label).evaluate((el) => {
          const animation = el.getAnimations()[0];
          const effect = animation.effect as KeyframeEffect;
          const timing = effect.getTiming();
          const tail = effect.getKeyframes()[3].computedOffset;
          animation.pause();
          animation.currentTime = timing.delay + tail * Number(timing.duration) + 150;
        });
        await expect(label).toHaveAttribute('data-scroll-state', 'tail');
        await expect(label).toHaveAttribute('data-fade', 'left');
        const viewport = (await label.boundingBox())!;
        const track = (await textTrack(label).boundingBox())!;
        expect(Math.abs(viewport.x + viewport.width - track.x - track.width)).toBeLessThan(1);
      };
      await objectName.hover();
      await finishTravel(objectName);
      expect(await panel.locator('header').boundingBox()).toEqual(headerBounds);
      expect(
        await page.evaluate('window.__INSPECTOR_CALLS__.filter(c=>c[0]==="body").length'),
      ).toBe(0);
      await page.mouse.move(600, 40);
      await expect(objectName).not.toHaveAttribute('data-scroll-state');
      await expect(textTrack(objectName)).toHaveCSS('transform', 'none');
      await page.keyboard.press('Tab');
      await objectName.focus();
      await expect(objectName).toHaveAttribute('data-scroll-state', 'forward');
      await panel.locator('[data-history-snapshot]').first().click();
      const savedName = panel
        .locator('[data-hover-scroll-label]')
        .filter({ hasText: LONG_HISTORY_NAME });
      await expect(savedName).toHaveAttribute('data-overflow', 'true');
      await savedName.hover();
      await finishTravel(savedName);
      expect(
        await page.evaluate('window.__INSPECTOR_CALLS__.filter(c=>c[0]==="body").length'),
      ).toBe(1);
      await panel.getByRole('tab', { name: 'Attachments', exact: true }).click();
      const file = panel.locator('[data-inspector-attachment]').first();
      const fileName = file.locator('[data-hover-scroll-label]');
      await expect(fileName).toHaveAttribute('data-overflow', 'true');
      const rowBounds = await file.boundingBox();
      const iconBounds = await file.locator('svg').first().boundingBox();
      const metadataBounds = await file.locator('time').boundingBox();
      await file.locator('svg').first().hover();
      await finishTravel(fileName);
      expect(await file.boundingBox()).toEqual(rowBounds);
      expect(await file.locator('svg').first().boundingBox()).toEqual(iconBounds);
      expect(await file.locator('time').boundingBox()).toEqual(metadataBounds);
      await page.mouse.move(600, 40);
      await expect(fileName).not.toHaveAttribute('data-scroll-state');
      await expect(textTrack(fileName)).toHaveCSS('transform', 'none');
      await page.keyboard.press('Tab');
      await file.focus();
      await expect(fileName).toHaveAttribute('data-scroll-state', 'forward');
      const oldTrack = await textTrack(fileName).elementHandle();
      await panel.getByRole('tab', { name: 'History', exact: true }).click();
      expect(await oldTrack!.evaluate((el) => el.isConnected)).toBe(false);
      await panel.getByRole('tab', { name: 'Attachments', exact: true }).click();
      const shortName = panel
        .locator('[data-inspector-attachment]')
        .nth(1)
        .locator('[data-hover-scroll-label]');
      await expect(shortName).toHaveAttribute('data-overflow', 'false');
      await shortName.hover();
      await expect(shortName).not.toHaveAttribute('data-scroll-state');
      await page.evaluate(() => {
        document.documentElement.dataset.highContrast = 'true';
      });
      await expect(fileName).toHaveAttribute('data-static', 'true');
      await file.hover();
      await expect(fileName).toHaveAttribute('title', LONG_FILE_NAME);
      await expect(fileName).not.toHaveAttribute('data-scroll-state');
      await expect(textTrack(fileName)).toHaveCSS('transform', 'none');
    });
  }
}

test('scrolling names retain one continuous timeline through stops, reversals and loops', async ({
  page,
}) => {
  await setup(page, 'windows', 'dark', true);
  await page.locator('[data-object-inspector-toggle]').click();
  const label = page.locator('[data-object-inspector] header [data-hover-scroll-label]');
  await label.hover();
  await expect(label).toHaveAttribute('data-scroll-state', 'forward');
  const result = await label.evaluate(async (el) => {
    const track = el.querySelector<HTMLElement>('[data-hover-scroll-track]')!;
    const animation = track.getAnimations()[0];
    const effect = animation.effect as KeyframeEffect;
    const timing = effect.getTiming();
    const duration = Number(timing.duration);
    const frames = effect.getKeyframes();
    const at = async (time: number) => {
      animation.currentTime = time;
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
      return {
        x: new DOMMatrix(getComputedStyle(track).transform).m41,
        phase: (el as HTMLElement).dataset.scrollState,
        sameAnimation: track.getAnimations().length === 1 && track.getAnimations()[0] === animation,
      };
    };
    animation.pause();
    const boundaries = [];
    for (const index of [0, 3, 4, 7, 8]) {
      const boundary = timing.delay + frames[index].computedOffset * duration;
      boundaries.push({ before: await at(boundary - 16), after: await at(boundary + 16) });
    }
    // 起步与停下有短缓动，正文仍在同一合成时间轴内连续前进。
    const start = await at(timing.delay);
    const first = await at(timing.delay + 16);
    const cruiseStart = await at(timing.delay + 500);
    const cruiseEnd = await at(timing.delay + 516);
    return {
      iterations: timing.iterations,
      boundaries,
      startStep: Math.abs(first.x - start.x),
      cruiseStep: Math.abs(cruiseEnd.x - cruiseStart.x),
    };
  });
  expect(result.iterations).toBe(Infinity);
  expect(result.boundaries.map(({ after }) => after.phase)).toEqual([
    'forward',
    'tail',
    'reverse',
    'rest',
    'forward',
  ]);
  for (const { before, after } of result.boundaries) {
    expect(before.sameAnimation && after.sameAnimation).toBe(true);
    expect(Math.abs(after.x - before.x)).toBeLessThan(1);
  }
  expect(result.cruiseStep).toBeGreaterThan(0.3);
  expect(result.startStep).toBeLessThan(result.cruiseStep / 4);
  await page.mouse.move(600, 40);
  await expect(label).not.toHaveAttribute('data-scroll-state');
  await expect(label.locator('[data-hover-scroll-track]')).toHaveCSS('transform', 'none');
  expect(
    await label.locator('[data-hover-scroll-track]').evaluate((el) => el.getAnimations().length),
  ).toBe(0);
});
