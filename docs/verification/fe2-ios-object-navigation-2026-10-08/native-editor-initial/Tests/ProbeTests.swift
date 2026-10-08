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
    func testCurrentProductionObjectAndMobileNavigation() {
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
        let fullName = app.textFields.matching(NSPredicate(format: "label BEGINSWITH %@", "姓名")).firstMatch
        XCTAssertTrue(fullName.exists, "Required public name field missing")
        fullName.tap(); fullName.typeText("Public Person\n")
        capture(app, "filled-identity-editor")
        let save = app.buttons["保存"]
        let cancelEdit = app.buttons["取消"]
        XCTAssertTrue(save.exists); XCTAssertTrue(cancelEdit.exists)
        for _ in 0..<8 {
            if save.isHittable && cancelEdit.isHittable && cancelEdit.frame.maxY <= home.frame.minY { break }
            app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.70)).press(forDuration: 0.05, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.30)))
        }
        XCTAssertTrue(save.isHittable, "Save must remain reachable by real content scrolling")
        XCTAssertTrue(cancelEdit.isHittable, "Cancel must remain reachable by real content scrolling")
        XCTAssertGreaterThanOrEqual(save.frame.minY, 110)
        XCTAssertLessThanOrEqual(save.frame.maxY, home.frame.minY)
        XCTAssertLessThanOrEqual(cancelEdit.frame.maxY, home.frame.minY)
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
        close.tap()
        XCTAssertTrue(object.exists); XCTAssertTrue(object.isHittable); XCTAssertTrue(create.exists)
        capture(app, "detail-closed-same-workspace")
        home.tap()
        XCTAssertTrue(app.buttons["展开"].waitForExistence(timeout: 15))
        let addPage = app.buttons["添加页面"]
        XCTAssertTrue(addPage.exists); XCTAssertTrue(addPage.isHittable); addPage.tap()
        let pageName = field(app, "页面名称")
        XCTAssertTrue(pageName.waitForExistence(timeout: 15)); XCTAssertTrue(pageName.isHittable)
        capture(app, "add-page-popover")
        let cancel = app.buttons["取消"]
        XCTAssertTrue(cancel.exists); XCTAssertTrue(cancel.isHittable); cancel.tap()
        XCTAssertFalse(pageName.exists)
        capture(app, "add-page-cancelled-home")
    }
}
