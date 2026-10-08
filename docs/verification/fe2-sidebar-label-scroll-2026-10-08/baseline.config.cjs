const { defineConfig } = require('/Users/zzc/PycharmProjects/SoloSoul/tauri/node_modules/@playwright/test');
module.exports = defineConfig({
 testDir:'/Users/zzc/PycharmProjects/SoloSoul/tauri/e2e',
 testMatch:['sidebar-tools.spec.ts','macos-titlebar.spec.ts','desktop-functional-colors.spec.ts'],
 timeout:45000, workers:2, retries:0, reporter:[['list'],['json',{outputFile:'/tmp/solosoul-sidebar-label-20261008/baseline-results.json'}]],
 outputDir:'/tmp/solosoul-sidebar-label-20261008/baseline-results',
 use:{baseURL:'http://localhost:1481',viewport:{width:1200,height:800}},
 projects:[{name:'chromium',use:{browserName:'chromium'}}],
 webServer:{command:'SOLOSOUL_VITE_HMR_PORT=1482 npm run dev -- --port 1481',cwd:'/Users/zzc/PycharmProjects/SoloSoul/tauri',url:'http://localhost:1481',reuseExistingServer:false,timeout:120000}
});
