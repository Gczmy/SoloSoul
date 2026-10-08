from pathlib import Path
ROOT=Path('/Users/zzc/PycharmProjects/SoloSoul')
helper='tauri/src-tauri/gen/android/app/src/androidTest/java/com/solosoul/app/AndroidHomeInstrumentedTest.kt'
original=(ROOT/helper).read_text()
addition=r'''
    private val safPickerChecks get() = InstrumentationRegistry.getArguments().getString("safPickerChecks") == "true"
    private val ownedSafUris = mutableListOf<android.net.Uri>()
    private fun nativeNodes(root: AccessibilityNodeInfo): List<AccessibilityNodeInfo> {
        val result = mutableListOf<AccessibilityNodeInfo>()
        fun visit(node: AccessibilityNodeInfo) {
            result.add(node)
            for (i in 0 until node.childCount) node.getChild(i)?.let { visit(it) }
        }
        visit(root)
        return result
    }
    private fun waitNative(label: String, predicate: (AccessibilityNodeInfo) -> Boolean): AccessibilityNodeInfo {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(25)
        do {
            instrumentation.uiAutomation.rootInActiveWindow?.let { if (predicate(it)) return it }
            Thread.sleep(150)
        } while (System.nanoTime() < deadline)
        val root = instrumentation.uiAutomation.rootInActiveWindow
        val visible = root?.let { nativeNodes(it).map { n -> "${n.viewIdResourceName}: ${n.text ?: n.contentDescription}" }.joinToString("\n") }
        throw AssertionError("$label: package=${root?.packageName}\n$visible")
    }
    private fun clickNative(node: AccessibilityNodeInfo) {
        var target: AccessibilityNodeInfo? = node
        repeat(6) {
            val current = target ?: return@repeat
            if (current.isClickable && current.isEnabled) {
                assertTrue("实际系统控件点击", current.performAction(AccessibilityNodeInfo.ACTION_CLICK))
                instrumentation.waitForIdleSync()
                return
            }
            target = current.parent
        }
        throw AssertionError("系统控件无可点击祖先: ${node.text} / ${node.contentDescription}")
    }
    private fun pickPublicDocument(name: String, index: Int) {
        var root = waitNative("实际系统文件选择器", { it.packageName?.toString()?.contains("documentsui") == true })
        File(evidence,"saf-picker-open-$index.png").outputStream().use { stream ->
            val frame=requireNotNull(instrumentation.uiAutomation.takeScreenshot());frame.compress(Bitmap.CompressFormat.PNG,100,stream);frame.recycle()
        }
        record(JSONObject().put("stage","saf-picker-open-$index").put("package",root.packageName.toString()).put("publicFile",name))
        val search = nativeNodes(root).firstOrNull {
            it.contentDescription?.toString() == "Search" || it.viewIdResourceName?.endsWith(":id/option_menu_search") == true
        } ?: throw AssertionError("系统选择器没有可见搜索入口")
        clickNative(search)
        root = waitNative("系统搜索输入", { nativeNodes(it).any { n -> n.isEditable } })
        val input = nativeNodes(root).first { it.isEditable }
        val args = android.os.Bundle().apply { putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE,name) }
        assertTrue("搜索仅公开夹具文件名",input.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT,args))
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_ENTER)
        root = waitNative("系统选择器真实文件搜索结果", { nativeNodes(it).any { n -> n.text?.toString() == name && !n.isEditable } })
        File(evidence,"saf-picker-result-$index.png").outputStream().use { stream ->
            val frame=requireNotNull(instrumentation.uiAutomation.takeScreenshot());frame.compress(Bitmap.CompressFormat.PNG,100,stream);frame.recycle()
        }
        clickNative(nativeNodes(root).first { it.text?.toString() == name && !it.isEditable })
        waitNative("选择文件后返回SoloSoul", { it.packageName?.toString() == "com.solosoul.app" })
        record(JSONObject().put("stage","saf-picker-selected-$index").put("publicFile",name).put("actualNativeSelection",true))
    }
    private fun seedActualSafFixtures(scenario: ActivityScenario<MainActivity>, files: List<Pair<File,String>>) {
        val resolver=instrumentation.targetContext.contentResolver
        val tag="SoloSoul-FE2-SAF-${java.util.UUID.randomUUID()}"
        for ((file,mime) in files) {
            val values=android.content.ContentValues().apply {
                put(android.provider.MediaStore.MediaColumns.DISPLAY_NAME,file.name)
                put(android.provider.MediaStore.MediaColumns.MIME_TYPE,mime)
                put(android.provider.MediaStore.MediaColumns.RELATIVE_PATH,"Download/$tag/")
                put(android.provider.MediaStore.MediaColumns.IS_PENDING,1)
            }
            val uri=requireNotNull(resolver.insert(android.provider.MediaStore.Downloads.EXTERNAL_CONTENT_URI,values))
            ownedSafUris.add(uri)
            requireNotNull(resolver.openOutputStream(uri)).use { out -> file.inputStream().use { it.copyTo(out) } }
            resolver.update(uri,android.content.ContentValues().apply { put(android.provider.MediaStore.MediaColumns.IS_PENDING,0) },null,null)
        }
        clickSelector(scenario,".android-object-detail-footer button[aria-label^='Attachments']")
        val panel="[data-macos-glass-backdrop][style*='z-index: 5100'] > [data-macos-glass='panel']"
        waitFor(scenario,"附件真实上传入口", "JSON.stringify({ready:!!document.querySelector(\"button[title='Upload']\")})") { it.optBoolean("ready") }
        for((index,entry) in files.withIndex()) {
            clickSelector(scenario,"button[title='Upload']")
            pickPublicDocument(entry.first.name,index+1)
            waitFor(scenario,"系统文件选择后附件列表可见", "JSON.stringify({ready:!!document.querySelector(\"button[aria-haspopup='dialog'][aria-label*='${entry.first.name}']\")})") { it.optBoolean("ready") }
        }
        js(scenario,"""(() => {window.fe2SafVerification={pending:true};(async()=>{
            const invoke=window.__TAURI_INTERNALS__.invoke;
            const accounts=await invoke('vault_list_accounts',{});
            if(accounts.length!==1||accounts[0].name!=='FE2 public visual test')throw Error('not owned synthetic account');
            const objects=await invoke('object_list',{accountId:accounts[0].id,filter:null});
            if(objects.length!==1||objects[0].name!=='FE2 public object')throw Error('not owned object');
            const rows=await invoke('attachment_list',{objectId:objects[0].id,showDeleted:false});
            window.fe2SafVerification={rows:rows.map(f=>({name:f.fileName,path:f.vaultPath,size:f.sizeBytes,source:f.srcPath}))};
        })().catch(e=>window.fe2SafVerification={error:String(e)});return JSON.stringify({started:true});})()""")
        val result=waitFor(scenario,"系统选择器导入真实加密附件", "JSON.stringify(window.fe2SafVerification||{})") { it.has("rows") || it.has("error") }
        assertFalse(result.toString(),result.has("error"))
        val rows=result.getJSONArray("rows");assertEquals(2,rows.length())
        for(i in 0 until rows.length()) {
            val row=rows.getJSONObject(i);assertTrue(row.toString(),row.getLong("size")>0)
            assertTrue("保存原始系统content URI",row.getString("source").startsWith("content://"))
            val magic=ByteArray(4);File(row.getString("path")).inputStream().use{assertEquals(4,it.read(magic))}
            assertEquals("SOLC",String(magic,Charsets.US_ASCII))
        }
        record(JSONObject().put("stage","preview-fixtures-encrypted").put("savedCount",2).put("encrypted",true)
            .put("source","Actual Android DocumentsUI selection through Upload UI and production attachment pipeline").put("actualSafPicker",true))
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor(scenario,"上传后回对象详情", "JSON.stringify({panel:!!document.querySelector(${JSONObject.quote(panel)}),detail:!!document.querySelector('[data-testid=object-detail-modal]')})") { !it.optBoolean("panel") && it.optBoolean("detail") }
    }
'''
variant=original.replace('    private val instrumentation get()',addition+'\n    private val instrumentation get()',1)
variant=variant.replace('        val fixtures = JSONArray()','        if(safPickerChecks) { seedActualSafFixtures(scenario,listOf(text to "text/plain",image to "image/png"));return }\n        val fixtures = JSONArray()',1)
variant=variant.replace('            dark && !stage.contains("after-back") -> "backup"','            !safPickerChecks && dark && !stage.contains("after-back") -> "backup"',1)
variant=variant.replace('            .put("directoryAttempts", directoryAttempts)', '            .put("actualSafPickerChecks",safPickerChecks)\n            .put("directoryAttempts", directoryAttempts)',1)
marker='        } finally {\n            writeReport()\n        }\n    }\n}'
assert marker in variant
variant=variant.replace(marker,'''        } finally {
            val cleanup=JSONArray()
            for(uri in ownedSafUris) {
                val deleted=instrumentation.targetContext.contentResolver.delete(uri,null,null)
                cleanup.put(JSONObject().put("deleted",deleted).put("uri",uri.toString()))
                assertEquals("Only test-owned public MediaStore entries cleaned",1,deleted)
            }
            if(safPickerChecks)record(JSONObject().put("stage","saf-public-fixtures-cleaned").put("entries",cleanup).put("count",cleanup.length()))
            writeReport()
        }
    }
}''',1)
Path('/tmp/solosoul-fe2-android-saf-probe-20261008.kt').write_text(variant)
build=Path('/tmp/solosoul-fe2-android-current-file-stage-test-build-20261008.py').read_text().replace('solosoul-fe2-android-current-file-stage-test-build-20261008','solosoul-fe2-android-saf-test-build-20261008')
build=build.replace("'tauri/src-tauri/gen/android/app/tauri.properties']","'tauri/src-tauri/gen/android/app/tauri.properties','"+helper+"']",1)
insertion=" print('Protected input snapshots verified',len(backups),flush=True)\n variant=Path('/tmp/solosoul-fe2-android-saf-probe-20261008.kt')\n shutil.copy2(variant,OUT/'AndroidHomeInstrumentedTest.kt')\n report['temporary_helper_sha256']=hashlib.sha256(variant.read_bytes()).hexdigest()\n shutil.copy2(variant,ROOT/"+repr(helper)+")"
build=build.replace(" print('Protected input snapshots verified',len(backups),flush=True)",insertion,1)
build=build.replace("'scope':'Current source ARM64 Android instrumentation APK; production APK separately frozen'","'scope':'Temporary actual SAF instrumentation variant; current production 87e493af APK remains frozen and unchanged'",1)
Path('/tmp/solosoul-fe2-android-saf-test-build-20261008.py').write_text(build)
runner=(ROOT/'tauri/scripts/android-home-native-regression.py').read_text()
runner=runner.replace('    report = {"serial": args.serial,','    report = {"actualSafPickerChecks": True, "serial": args.serial,',1)
runner=runner.replace('"-e", "scenario", "accounts" if args.scenario == "cold" else args.scenario,','"-e", "safPickerChecks", "true",\n                     "-e", "scenario", "accounts" if args.scenario == "cold" else args.scenario,',1)
runner=runner.replace("notification = 'saved' if kind == 'viewer' else 'backup' if dark and 'after-back' not in stage else 'none'","notification = 'saved' if kind == 'viewer' else 'none'",1)
marker='        report["stages"] = stages'
runner=runner.replace(marker,'''        if not evidence.get('actualSafPickerChecks'):
            raise RuntimeError('Missing actual SAF mode binding')
        rows=evidence['records']
        for index in [1,2]:
            opened=next(row for row in rows if row['stage']==f'saf-picker-open-{index}')
            selected=next(row for row in rows if row['stage']==f'saf-picker-selected-{index}')
            if 'documentsui' not in opened.get('package','') or selected.get('actualNativeSelection') is not True:
                raise RuntimeError('Missing actual Android system picker action')
        fixtures=next(row for row in rows if row['stage']=='preview-fixtures-encrypted')
        cleanup=next(row for row in rows if row['stage']=='saf-public-fixtures-cleaned')
        if fixtures.get('actualSafPicker') is not True or cleanup.get('count')!=2 or any(row.get('deleted')!=1 for row in cleanup['entries']):
            raise RuntimeError('Missing actual SAF imports or public fixture cleanup')
'''+marker,1)
Path('/tmp/solosoul-fe2-android-saf-native-runner-20261008.py').write_text(runner)
print('Temporary SAF test and guarded drivers prepared; product source unchanged.')
