import { test, expect, type Page } from '/Users/zzc/PycharmProjects/SoloSoul/tauri/node_modules/@playwright/test';
import { setupTauriMock, login } from '/Users/zzc/PycharmProjects/SoloSoul/tauri/e2e/fixtures/auth';
async function setupPreview(page: Page, platform = 'macos', kind = 'text', theme = 'light') {
  // 混合平台文件仅调整桌面预览，保留Android/iOS视口与触摸上下文。
  if (platform === 'macos' || platform === 'windows') {
    await page.setViewportSize({ width: 1280, height: 720 });
  }
  await setupTauriMock(page);
  await page.addInitScript(
    ({ platform, kind, theme }) => {
      const prefs = {
        theme,
        language: 'en-US',
        hasSeenOnboarding: true,
        autoLockTimeoutMinutes: 0,
        // 本文件显式创建待测提醒；隔离自动备份提醒，避免两个通知之间的空隙干扰采样。
        lastBackupReminderAt: Date.now(),
      };
      const photo = kind === 'image';
      const fileName = photo
        ? 'Travel photo.png'
        : kind === 'pdf'
          ? 'Travel report.pdf'
          : 'A long attachment name for checking title truncation in a narrow window.txt';
      const attachment = {
        id: 'preview-file',
        objectId: 'preview-object',
        fileName,
        mimeType: photo ? 'image/png' : kind === 'pdf' ? 'application/pdf' : 'text/plain',
        sizeBytes: 128,
        createdAt: '2026-09-16',
        vaultPath: `/mock-vault/${fileName}`,
      };
      const imageUrl =
        'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAUAAAADwCAIAAAD+Tyo8AAAC80lEQVR4nO3VwQnCUBBAwSiC5egp5uzBoiwrjaSL1JEqPuGFmQKWZeGxt/3zmriu5f8+ewUGuo8cDowlYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUPY4zuvZ+/AQNvz574X5gNDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMYQKGMAFDmIAhTMAQJmAIEzCECRjCBAxhAoYwAUOYgCFMwBAmYAgTMIQJGMIEDGEChjABQ5iAIUzAECZgCBMwhAkYwgQMU9cBp0QGVhM6xLwAAAAASUVORK5CYII=';
      const layout = {
        platform,
        titlebarHeight: platform === 'macos' ? 52 : 0,
        trafficLightsRight: platform === 'macos' ? 79 : 0,
      };
      Object.assign(window, {
        __MOCK_PLATFORM__: platform,
        __previewLayout: layout,
        __previewAppearanceCalls: 0,
        __E2E_MOCKS__: {
          ui_get_preferences: () => prefs,
          user_data_get_preferences: () => prefs,
          vault_check_directory: () => true,
          set_titlebar_color: () => {
            (window as any).__previewAppearanceCalls++;
            return {
              ...layout,
              material:
                platform === 'macos' ? 'liquid-glass' : platform === 'windows' ? 'mica' : 'solid',
              reduceMotion: false,
              highContrast: false,
            };
          },
          get_window_layout: () => layout,
          set_titlebar_controls: ({ regions }: any) => {
            (window as any).__previewRegions = regions;
          },
          attachment_count_stats: () => ({ attachmentCount: 1, photoCount: photo ? 1 : 0 }),
          attachment_list: () => [attachment],
          object_get: () => ({
            id: attachment.objectId,
            name: 'Travel documents',
            typeId: 'travel',
            properties: {},
          }),
          attachment_list_all: () => ({
            pages: [
              {
                pageName: 'travel',
                objects: [
                  {
                    objectId: attachment.objectId,
                    objectName: 'Travel documents',
                    attachments: [attachment],
                  },
                ],
              },
            ],
            trashPages: [],
          }),
          fs_read_file_as_text: () => 'Preview test content',
          fs_read_file_as_data_url: () =>
            kind === 'pdf' ? `data:application/pdf;base64,${btoa('%PDF-1.4\n%%EOF')}` : imageUrl,
          fs_read_image_preview: () => imageUrl,
        },
      });
      localStorage.setItem('i18nextLng', 'en-US');
    },
    { platform, kind, theme },
  );
  await login(page);
  await page.evaluate(() => {
    history.pushState(
      { ...history.state, idx: history.state.idx + 1, usr: null },
      '',
      '/settings/attachments',
    );
    window.dispatchEvent(new PopStateEvent('popstate', { state: history.state }));
  });
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Global Attachment Manager');
}


for (const theme of ['dark', 'light']) {
 test(`photo toolbar drawing through zoom ${theme}`, async ({page}) => {
  await setupPreview(page, 'macos', 'image', theme);
  await page.getByRole('button', {name: /Photo Album/}).click();
  const album=page.getByTestId('photo-album-overlay');
  await album.getByRole('button', {name:'Travel photo.png',exact:true}).click();
  const viewer=page.getByTestId('photo-viewer');
  const toolbar=viewer.locator('[data-preview-zoom-controls]');
  await expect(toolbar).toBeVisible();
  const captures=[];
  for (const step of ['initial','zoom-in','zoom-in-again','fit']) {
   if (step==='zoom-in'||step==='zoom-in-again') await toolbar.getByRole('button',{name:'Zoom In',exact:true}).click();
   if (step==='fit') await toolbar.getByRole('button',{name:'Fit to window',exact:true}).click();
   await page.mouse.move(400,200);
   await expect(toolbar.locator('svg')).toHaveCount(3);
   const bounds=await toolbar.boundingBox();
   const capture=await page.screenshot({path:test.info().outputPath(`${step}.png`)});
   captures.push({step,bounds,percentage:await toolbar.locator('span').innerText(),bytes:capture.length});
  }
  await test.info().attach('drawing-metadata',{body:JSON.stringify(captures,null,2),contentType:'application/json'});
 });
}
