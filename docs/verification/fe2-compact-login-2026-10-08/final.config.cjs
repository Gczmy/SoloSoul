const { defineConfig } = require('/Users/zzc/PycharmProjects/SoloSoul/tauri/node_modules/@playwright/test');
module.exports = defineConfig({
 testDir:'/Users/zzc/PycharmProjects/SoloSoul/tauri/e2e',
 testMatch:['macos-glass.spec.ts','macos-preview-titlebar.spec.ts','login-method-layout.spec.ts'],
 timeout:45000, workers:2, retries:0, reporter:[['list'],['json',{outputFile:'/tmp/solosoul-login-no-underlay-20261008/final-results.json'}]],
 outputDir:'/tmp/solosoul-login-no-underlay-20261008/final-results',
 use:{baseURL:'http://localhost:1485',viewport:{width:1200,height:800}},
 projects:[{name:'chromium',use:{browserName:'chromium'}},{name:'webkit',use:{browserName:'webkit'}}],
 webServer:{command:'SOLOSOUL_VITE_HMR_PORT=1486 npm run dev -- --port 1485',cwd:'/Users/zzc/PycharmProjects/SoloSoul/tauri',url:'http://localhost:1485',reuseExistingServer:false,timeout:120000}
});
