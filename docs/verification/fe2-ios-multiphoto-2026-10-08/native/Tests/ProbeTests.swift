import XCTest
final class ProbeTests: XCTestCase {
    private func capture(_ app: XCUIApplication, _ name: String) {
        let image = XCTAttachment(screenshot: app.screenshot())
        image.name = name; image.lifetime = .keepAlways; add(image)
        let tree = XCTAttachment(string: app.debugDescription)
        tree.name = name + "-hierarchy"; tree.lifetime = .keepAlways; add(tree)
    }
    private func field(_ app: XCUIApplication, _ name: String) -> XCUIElement {
        let secure = app.secureTextFields[name]
        return secure.exists ? secure : app.textFields[name]
    }
    func testCurrentProductionNativeFilesAndPreviews() {
        continueAfterFailure = false
        let app = XCUIApplication(bundleIdentifier: "com.solosoul.app")
        app.launch()
        defer { capture(app, "final-state"); app.terminate() }
        for _ in 0..<4 {
            let next = app.buttons["下一步"]
            XCTAssertTrue(next.waitForExistence(timeout: 30)); XCTAssertTrue(next.isHittable)
            next.tap()
        }
        let done = app.buttons["完成"]
        XCTAssertTrue(done.waitForExistence(timeout: 15)); done.tap()
        let createNew = app.buttons["不，创建新账户"]
        XCTAssertTrue(createNew.waitForExistence(timeout: 30)); createNew.tap()
        let name = field(app, "账户名称")
        XCTAssertTrue(name.waitForExistence(timeout: 30))
        capture(app, "current-registration")
        name.tap(); name.typeText("FE2 iOS public account")
        let password = field(app, "主密码")
        XCTAssertTrue(password.exists); password.tap(); password.typeText("FE2-public-test-2026!")
        let confirmation = field(app, "确认密码")
        XCTAssertTrue(confirmation.exists); confirmation.tap(); confirmation.typeText("FE2-public-test-2026!\n")
        let home = app.buttons["首页"]
        XCTAssertTrue(home.waitForExistence(timeout: 60), "Production account creation did not reach home")
        XCTAssertTrue(home.isHittable)
        capture(app, "current-created-home")
        // Preserve the real reminder; wait for it to expire before testing the toolbar.
        Thread.sleep(forTimeInterval: 10)
        let expand = app.buttons["展开"]
        XCTAssertTrue(expand.exists); XCTAssertTrue(expand.isHittable); expand.tap()
        capture(app, "expanded-mobile-navigation")
        let collapse = app.buttons["收起"]
        XCTAssertTrue(collapse.exists); XCTAssertTrue(collapse.isHittable); collapse.tap()
        let identity = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "身份个人信息")).firstMatch
        XCTAssertTrue(identity.waitForExistence(timeout: 15)); XCTAssertTrue(identity.isHittable); identity.tap()
        capture(app, "empty-identity-workspace")
        let create = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@ OR label == %@", "+", "新建")).firstMatch
        XCTAssertTrue(create.waitForExistence(timeout: 20), "Workspace create control missing")
        XCTAssertTrue(create.isHittable); create.tap()
        capture(app, "identity-template-editor")
        let template = app.switches["身份信息"]
        XCTAssertTrue(template.waitForExistence(timeout: 20)); XCTAssertTrue(template.isHittable); template.tap()
        let objectName = field(app, "对象名称")
        XCTAssertTrue(objectName.waitForExistence(timeout: 15)); objectName.tap(); objectName.typeText("FE2 iOS public identity")
        capture(app, "object-name-with-keyboard")
        XCTAssertTrue(app.keyboards.firstMatch.waitForExistence(timeout: 5), "Real software keyboard must remain open")
        XCTAssertGreaterThanOrEqual(app.otherElements["横幅"].firstMatch.frame.minY, 0, "Object-name focus must keep AppBar onscreen")
        let fullName = app.textFields.matching(NSPredicate(format: "label BEGINSWITH %@", "姓名")).firstMatch
        XCTAssertTrue(fullName.exists, "Required public name field missing")
        for step in 0..<24 {
            let main = app.otherElements["主要"].firstMatch
            let top = max(110, main.frame.minY + 8)
            let bottom = min(home.frame.minY, main.frame.maxY) - 8
            let frame = fullName.frame
            capture(app, "public-name-scroll-\(step)")
            if frame.minY >= top && frame.maxY <= bottom { break }
            XCTAssertGreaterThan(bottom - top, 40)
            // 实际触控小步校正；越过目标后反向滑回，不持续朝同一方向滑动。
            let upwards = frame.maxY > bottom
            let missing = upwards ? frame.maxY - bottom : top - frame.minY
            let distance = min(40, max(20, missing + 8))
            let middle = (top + bottom) / 2
            let origin = app.coordinate(withNormalizedOffset: .zero)
            origin.withOffset(CGVector(dx: app.frame.midX, dy: middle))
                .press(forDuration: 0.15, thenDragTo: origin.withOffset(CGVector(dx: app.frame.midX, dy: middle + (upwards ? -distance : distance))))
            Thread.sleep(forTimeInterval: 0.3)
        }
        capture(app, "public-name-before-focus")
        let mainBeforeFocus = app.otherElements["主要"].firstMatch
        XCTAssertGreaterThanOrEqual(fullName.frame.minY, max(110, mainBeforeFocus.frame.minY))
        XCTAssertLessThanOrEqual(fullName.frame.maxY, min(home.frame.minY, mainBeforeFocus.frame.maxY))
        let fieldCenter = fullName.frame
        app.coordinate(withNormalizedOffset: .zero).withOffset(CGVector(dx: fieldCenter.midX, dy: fieldCenter.midY)).tap()
        capture(app, "public-name-focused-before-typing")
        XCTAssertTrue(app.keyboards.firstMatch.waitForExistence(timeout: 5))
        fullName.typeText("Public Person\n")
        XCTAssertEqual(fullName.value as? String, "Public Person", "Actual touch must focus field and accept real keyboard input")
        capture(app, "filled-identity-editor")
        let header = app.otherElements["横幅"].firstMatch
        XCTAssertTrue(header.exists)
        XCTAssertGreaterThanOrEqual(header.frame.minY, 0, "Keyboard must not push AppBar above screen")
        XCTAssertTrue(app.buttons["返回"].isHittable)
        let toolbar = app.toolbars.firstMatch
        let keyboardTopDuringEdit = toolbar.exists ? toolbar.frame.minY : app.keyboards.firstMatch.frame.minY
        XCTAssertGreaterThanOrEqual(fullName.frame.minY, header.frame.maxY)
        XCTAssertLessThanOrEqual(fullName.frame.maxY, keyboardTopDuringEdit)

        let save = app.buttons["保存"]
        let cancelEdit = app.buttons["取消"]
        XCTAssertTrue(save.exists); XCTAssertTrue(cancelEdit.exists)
        for _ in 0..<8 {
            if save.isHittable && cancelEdit.isHittable && cancelEdit.frame.maxY <= home.frame.minY { break }
            let main = app.otherElements["主要"].firstMatch
            XCTAssertTrue(main.exists)
            let top = max(110, main.frame.minY + 12)
            let bottom = min(home.frame.minY, main.frame.maxY) - 12
            XCTAssertGreaterThan(bottom - top, 40)
            let origin = app.coordinate(withNormalizedOffset: .zero)
            let start = origin.withOffset(CGVector(dx: app.frame.midX, dy: top + (bottom - top) * 0.85))
            let end = origin.withOffset(CGVector(dx: app.frame.midX, dy: top + (bottom - top) * 0.15))
            start.press(forDuration: 0.05, thenDragTo: end)
        }
        capture(app, "editor-after-real-viewport-scroll")
        XCTAssertTrue(save.isHittable, "Save must remain reachable by real content scrolling")
        XCTAssertTrue(cancelEdit.isHittable, "Cancel must remain reachable by real content scrolling")
        XCTAssertGreaterThanOrEqual(save.frame.minY, 110)
        XCTAssertLessThanOrEqual(save.frame.maxY, home.frame.minY)
        XCTAssertLessThanOrEqual(cancelEdit.frame.maxY, home.frame.minY)
        XCTAssertGreaterThanOrEqual(header.frame.minY, 0, "Content scroll must not move AppBar offscreen")
        XCTAssertTrue(app.buttons["返回"].isHittable)
        capture(app, "editor-save-reachable")
        save.tap()
        let object = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "FE2 iOS public identity")).firstMatch
        XCTAssertTrue(object.waitForExistence(timeout: 30), "Production object save did not return to its workspace")
        XCTAssertTrue(object.isHittable)
        capture(app, "populated-identity-workspace")
        object.tap()
        let close = app.buttons["关闭"].firstMatch
        XCTAssertTrue(close.waitForExistence(timeout: 20)); XCTAssertTrue(close.isHittable)
        XCTAssertTrue(app.staticTexts["Public Person"].exists, "Saved public field was not present in actual detail")
        capture(app, "identity-detail")
        let attachments = app.buttons["附件"]
        XCTAssertTrue(attachments.waitForExistence(timeout: 15)); XCTAssertTrue(attachments.isHittable); attachments.tap()
        capture(app, "empty-attachments-panel")
        selectFixture(app, "FE2-public-note", "note")
        XCTAssertTrue(app.staticTexts["FE2-public-note.txt"].waitForExistence(timeout: 30), "Real import must create attachment row")
        capture(app, "imported-public-note")
        previewText(app, "light")
        selectFixture(app, "FE2-public-photo", "photo")
        XCTAssertTrue(app.staticTexts["FE2-public-photo.png"].waitForExistence(timeout: 30), "Real PNG import must create attachment row")
        capture(app, "imported-public-photo")
        selectFixture(app, "FE2-public-second", "second-photo")
        XCTAssertTrue(app.staticTexts["FE2-public-second.png"].waitForExistence(timeout: 30), "Real second PNG import must create attachment row")
        capture(app, "imported-public-second-photo")
        previewAlbum(app, "light")
        closeAttachments(app)
        capture(app, "light-returned-to-object-detail")
        XCTAssertTrue(app.staticTexts["Public Person"].exists)
        app.buttons["关闭"].firstMatch.tap()
        XCTAssertTrue(object.waitForExistence(timeout: 15)); capture(app, "light-returned-to-workspace")
        app.buttons["设置"].tap()
        let appearance = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "主题与外观")).firstMatch
        XCTAssertTrue(appearance.waitForExistence(timeout: 15)); XCTAssertTrue(appearance.isHittable); appearance.tap()
        let dark = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", "深色 ·")).firstMatch
        XCTAssertTrue(dark.waitForExistence(timeout: 15)); XCTAssertTrue(dark.isHittable); dark.tap()
        Thread.sleep(forTimeInterval: 1.5); capture(app, "actual-dark-theme-selected")
        home.tap(); XCTAssertTrue(identity.waitForExistence(timeout: 15)); identity.tap()
        XCTAssertTrue(object.waitForExistence(timeout: 15)); object.tap()
        XCTAssertTrue(app.buttons["附件"].waitForExistence(timeout: 15)); app.buttons["附件"].tap()
        XCTAssertTrue(app.staticTexts["FE2-public-note.txt"].waitForExistence(timeout: 15))
        capture(app, "dark-nonempty-attachments")
        previewText(app, "dark")
        previewAlbum(app, "dark")
        closeAttachments(app)
        XCTAssertTrue(app.staticTexts["Public Person"].exists); capture(app, "dark-returned-to-object-detail")
        app.buttons["关闭"].firstMatch.tap()
        XCTAssertTrue(object.waitForExistence(timeout: 15)); capture(app, "dark-returned-to-workspace")
    }
    private func selectFixture(_ app: XCUIApplication, _ name: String, _ stage: String) {
        let upload = app.buttons["上传"]
        XCTAssertTrue(upload.waitForExistence(timeout: 15)); XCTAssertTrue(upload.isHittable); upload.tap()
        Thread.sleep(forTimeInterval: 2); capture(app, stage + "-native-picker-open")
        let file = app.cells.matching(NSPredicate(format: "label CONTAINS %@", name))
        if !file.firstMatch.exists {
            let localRoot = app.otherElements.matching(NSPredicate(format: "identifier BEGINSWITH %@", "DOC.browsingRoot Source: com.apple.FileProvider.LocalStorage"))
            if !localRoot.firstMatch.exists {
                let browse = app.buttons.matching(NSPredicate(format: "label == %@ AND identifier != %@", "浏览", "BackButton"))
                XCTAssertTrue(browse.firstMatch.waitForExistence(timeout: 15)); XCTAssertEqual(browse.count, 1)
                XCTAssertTrue(browse.firstMatch.isHittable); browse.firstMatch.tap()
                if !localRoot.firstMatch.exists {
                    let local = app.cells.matching(NSPredicate(format: "label CONTAINS %@", "iPhone"))
                    XCTAssertTrue(local.firstMatch.waitForExistence(timeout: 15)); XCTAssertEqual(local.count, 1)
                    XCTAssertTrue(local.firstMatch.isHittable); local.firstMatch.tap()
                }
            }
            XCTAssertTrue(localRoot.firstMatch.waitForExistence(timeout: 15))
            capture(app, stage + "-native-picker-local")
            let folder = app.cells.matching(NSPredicate(format: "label CONTAINS %@", "SoloSoul FE2 Files"))
            XCTAssertTrue(folder.firstMatch.waitForExistence(timeout: 15)); XCTAssertEqual(folder.count, 1)
            XCTAssertTrue(folder.firstMatch.isHittable); folder.firstMatch.tap()
        }
        capture(app, stage + "-native-picker-public-files")
        XCTAssertTrue(file.firstMatch.waitForExistence(timeout: 15)); XCTAssertEqual(file.count, 1)
        XCTAssertTrue(file.firstMatch.isHittable); file.firstMatch.tap()
        capture(app, stage + "-native-picker-file-selected")
    }
    private func topBack(_ app: XCUIApplication) {
        let label = app.buttons["返回照片集"].exists ? "返回照片集" : "返回"
        let buttons = app.buttons.matching(NSPredicate(format: "label == %@", label)).allElementsBoundByIndex.filter { $0.frame.minY < 150 && $0.isHittable }
        XCTAssertFalse(buttons.isEmpty)
        let button = buttons.min { $0.frame.minY < $1.frame.minY }!
        XCTAssertGreaterThanOrEqual(button.frame.minY, 60)
        XCTAssertGreaterThanOrEqual(button.frame.width, 44); XCTAssertGreaterThanOrEqual(button.frame.height, 44)
        button.tap()
    }
    private func previewText(_ app: XCUIApplication, _ mode: String) {
        let name = app.staticTexts["FE2-public-note.txt"]
        XCTAssertTrue(name.exists)
        let candidates = app.buttons.matching(NSPredicate(format: "label == %@", "预览")).allElementsBoundByIndex.filter { $0.isHittable && $0.frame.minY >= name.frame.minY }
        let preview = candidates.min { $0.frame.minY < $1.frame.minY }
        XCTAssertNotNil(preview); XCTAssertLessThan(preview!.frame.minY - name.frame.minY, 100)
        preview!.tap()
        let text = app.staticTexts.matching(NSPredicate(format: "label CONTAINS %@", "Readable text in light and dark themes."))
        XCTAssertTrue(text.firstMatch.waitForExistence(timeout: 25), "Actual decrypted public TXT must be readable")
        XCTAssertTrue(text.firstMatch.isHittable)
        capture(app, mode + "-public-text-preview")
        topBack(app)
        XCTAssertTrue(app.buttons["上传"].waitForExistence(timeout: 15)); XCTAssertTrue(name.exists)
        capture(app, mode + "-text-returned-to-attachments")
    }
    private func previewAlbum(_ app: XCUIApplication, _ mode: String) {
        let album = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "照片集")).firstMatch
        XCTAssertTrue(album.waitForExistence(timeout: 15)); XCTAssertTrue(album.isHittable); album.tap()
        let first = app.buttons["FE2-public-photo.png"]
        let second = app.buttons["FE2-public-second.png"]
        XCTAssertTrue(first.waitForExistence(timeout: 25)); XCTAssertTrue(first.isHittable)
        XCTAssertTrue(second.waitForExistence(timeout: 25)); XCTAssertTrue(second.isHittable)
        capture(app, mode + "-two-photo-album"); first.tap()
        let reset = app.buttons["适应窗口"]
        XCTAssertTrue(reset.waitForExistence(timeout: 25)); XCTAssertTrue(reset.isHittable)
        let original = app.images["FE2-public-photo.png"]
        XCTAssertTrue(original.waitForExistence(timeout: 15))
        let counter = app.otherElements.matching(NSPredicate(format: "label MATCHES %@", "[12] / 2")).firstMatch
        XCTAssertTrue(counter.waitForExistence(timeout: 15))
        let initialCounter = counter.label
        XCTAssertEqual(app.buttons.matching(NSPredicate(format: "label == %@", "上一个")).count, 1)
        XCTAssertEqual(app.buttons.matching(NSPredicate(format: "label == %@", "下一个")).count, 1)
        for label in ["上一个", "下一个", "放大", "缩小", "适应窗口"] {
            let button = app.buttons[label]
            XCTAssertTrue(button.isHittable)
            XCTAssertGreaterThanOrEqual(button.frame.width, 44)
            XCTAssertGreaterThanOrEqual(button.frame.height, 44)
            XCTAssertLessThanOrEqual(button.frame.maxY, 840)
        }
        capture(app, mode + "-first-photo-fit")
        // 真实横向触控来自图像中部，不使用网页脚本或直接更改查看索引。
        let imageRect = original.frame
        let origin = app.coordinate(withNormalizedOffset: .zero)
        origin.withOffset(CGVector(dx: imageRect.minX + imageRect.width * 0.8, dy: imageRect.midY))
            .press(forDuration: 0.1, thenDragTo: origin.withOffset(CGVector(dx: imageRect.minX + imageRect.width * 0.2, dy: imageRect.midY)))
        let secondImage = app.images["FE2-public-second.png"]
        XCTAssertTrue(secondImage.waitForExistence(timeout: 15), "Native left drag must show second photo")
        XCTAssertTrue(reset.waitForExistence(timeout: 15))
        XCTAssertNotEqual(counter.label, initialCounter)
        capture(app, mode + "-native-swipe-second-photo")
        app.buttons["上一个"].tap()
        XCTAssertTrue(original.waitForExistence(timeout: 15)); XCTAssertTrue(reset.waitForExistence(timeout: 15))
        XCTAssertEqual(counter.label, initialCounter)
        capture(app, mode + "-previous-button-first-photo")
        app.buttons["下一个"].tap()
        XCTAssertTrue(secondImage.waitForExistence(timeout: 15)); XCTAssertTrue(reset.waitForExistence(timeout: 15))
        capture(app, mode + "-next-button-second-photo")
        let fitWidth = secondImage.frame.width
        app.buttons["放大"].tap()
        let zoomed = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in secondImage.exists && secondImage.frame.width > fitWidth * 1.1 }, object: nil)
        XCTAssertEqual(XCTWaiter.wait(for: [zoomed], timeout: 10), .completed, "Actual zoom tap must increase image size")
        XCTAssertNotEqual(counter.label, initialCounter)
        capture(app, mode + "-second-photo-zoomed")
        app.buttons["缩小"].tap()
        let reduced = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in secondImage.exists && abs(secondImage.frame.width - fitWidth) < 2 }, object: nil)
        XCTAssertEqual(XCTWaiter.wait(for: [reduced], timeout: 10), .completed)
        app.buttons["放大"].tap(); app.buttons["适应窗口"].tap()
        let resetSize = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in secondImage.exists && abs(secondImage.frame.width - fitWidth) < 2 }, object: nil)
        XCTAssertEqual(XCTWaiter.wait(for: [resetSize], timeout: 10), .completed)
        capture(app, mode + "-second-photo-fit-restored")
        topBack(app)
        XCTAssertTrue(first.waitForExistence(timeout: 15)); XCTAssertTrue(second.exists); XCTAssertFalse(reset.exists)
        capture(app, mode + "-viewer-returned-to-two-photo-album")
        topBack(app)
        XCTAssertTrue(app.buttons["上传"].waitForExistence(timeout: 15)); XCTAssertTrue(app.staticTexts["FE2-public-second.png"].exists)
        capture(app, mode + "-two-photo-album-returned-to-attachments")
    }
    private func closeAttachments(_ app: XCUIApplication) {
        let upload = app.buttons["上传"]
        XCTAssertTrue(upload.exists)
        let close = app.buttons.matching(NSPredicate(format: "label == %@", "关闭")).allElementsBoundByIndex.filter { abs($0.frame.midY - upload.frame.midY) < 2 && $0.isHittable }
        XCTAssertEqual(close.count, 1); close[0].tap()
        XCTAssertFalse(upload.exists, "Closing attachments must leave object detail open")
    }
}
