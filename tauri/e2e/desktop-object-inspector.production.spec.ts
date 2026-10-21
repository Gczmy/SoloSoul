import { expect, test, type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { login } from './fixtures/auth';

async function setup(page: Page, platform: 'windows' | 'macos', theme: 'light' | 'dark') {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.addInitScript({
    content:
      readFileSync('e2e/fixtures/tauriMock.js', 'utf8') +
      `
    window.__MOCK_PLATFORM__ = '${platform}';
    localStorage.setItem('i18nextLng','en-US');
    const prefs={theme:'${theme}',accentColor:'ocean',hasSeenOnboarding:true,defaultLightTheme:'warm-stone',defaultDarkTheme:'warm-stone-dark'};
    localStorage.setItem('solosoul_ui_prefs',JSON.stringify(prefs));
    const objects=['one','two','empty'].map((id,i)=>({id,name:i===0?'Passport for navigation':i===1?'Second record':'Empty record',
      typeId:'identity',sectionType:'identity',sensitivityLevel:'public',createdAt:'2026-10-20',updatedAt:'2026-10-20',
      isDeleted:false,templateId:null,templateType:null,tags:[],hasAttachments:false,
      properties:{__fields:{field:{name:'Public field',type:'text',sensitivityLevel:'public'}},field:'Editor body '+id},propertyLabels:{field:'public'}}));
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
        return {name:'Snapshot '+snapshotId,properties:{public:'PUBLIC '+snapshotId,secret:'PRIVATE '+snapshotId},propertyLabels:{public:'public',secret:'sensitive'}};
      },
      attachment_list:({objectId,showDeleted})=>{
        window.__INSPECTOR_CALLS__.push(['attachments',objectId,showDeleted]);
        return objectId==='empty'?[]:Array.from({length:36},(_,i)=>({id:objectId+'-file-'+i,objectId,
          fileName:objectId+' attachment '+i+' with a long descriptive name for the supporting document.pdf',mimeType:'application/pdf',
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
    .getByRole('button', { name: 'Passport for navigation', exact: true })
    .click();
  await expect(page).toHaveURL(/\/editor\/one$/);
  await expect(page.getByLabel('Object Name', { exact: true })).toHaveValue(
    'Passport for navigation',
  );
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
      await panel.locator('[data-history-snapshot]').first().click();
      await expect(panel.getByText('PUBLIC one-snap-31', { exact: true })).toBeVisible();
      await expect(panel.getByText('PRIVATE one-snap-31', { exact: true })).toHaveCount(0);
      await panel.getByRole('button', { name: 'Click to reveal', exact: true }).click();
      await expect(panel.getByText('PRIVATE one-snap-31', { exact: true })).toBeVisible();
      const scroller = panel.locator('[data-object-inspector-scroll]');
      const header = await panel.locator('header').boundingBox();
      const editorScroll = await content.evaluate((el) => el.scrollTop);
      await scroller.evaluate((el) => (el.scrollTop = el.scrollHeight));
      expect(await content.evaluate((el) => el.scrollTop)).toBe(editorScroll);
      expect(await panel.locator('header').boundingBox()).toEqual(header);
      await panel.getByRole('button', { name: 'Load more', exact: true }).click();
      await expect(panel.locator('[data-history-snapshot]')).toHaveCount(32);
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
