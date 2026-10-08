const { defineConfig } = require('/Users/zzc/PycharmProjects/SoloSoul/tauri/node_modules/@playwright/test');
module.exports = defineConfig({
 testDir:'/Users/zzc/PycharmProjects/SoloSoul/tauri/e2e',
 testMatch:['sidebar-label-scroll.spec.ts'],
 timeout:45000, workers:2, retries:0, reporter:[['list'],['json',{outputFile:'/tmp/solosoul-sidebar-label-20261008/results.json'}]],
 outputDir:'/tmp/solosoul-sidebar-label-20261008/results',
 use:{baseURL:'http://localhost:1473',viewport:{width:1200,height:800}},
 projects:[{name:'chromium',use:{browserName:'chromium'}},{name:'webkit',use:{browserName:'webkit'}}],
 webServer:{command:'SOLOSOUL_VITE_HMR_PORT=1474 npm run dev -- --port 1473',cwd:'/Users/zzc/PycharmProjects/SoloSoul/tauri',url:'http://localhost:1473',reuseExistingServer:false,timeout:120000}
});
