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
    func testCurrentProductionControlFontsColdRestartAndThemes() {
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
        let lock = app.buttons["锁定保险库"]
        XCTAssertTrue(lock.waitForExistence(timeout: 30)); XCTAssertTrue(lock.isHittable); lock.tap()
        let unlock = app.buttons["解锁"]
        XCTAssertTrue(unlock.waitForExistence(timeout: 30), "Lock did not reach unlock form")
        let loginPassword = field(app, "输入密码")
        XCTAssertTrue(loginPassword.waitForExistence(timeout: 30), "Lock did not reach password login")
        capture(app, "current-locked-login")
        loginPassword.tap(); loginPassword.typeText("FE2-public-test-2026!\n")
        XCTAssertTrue(home.waitForExistence(timeout: 60), "First password submission did not return to home")
        XCTAssertTrue(home.isHittable)
        capture(app, "current-unlocked-home")
        let keyboardBefore = XCTAttachment(string: "keyboards=\(app.keyboards.count)\n" + XCUIApplication(bundleIdentifier: "com.apple.springboard").debugDescription)
        keyboardBefore.name = "system-ui-after-password"; keyboardBefore.lifetime = .keepAlways; add(keyboardBefore)
        app.terminate(); app.launch()
        let coldPassword = field(app, "输入密码")
        XCTAssertTrue(coldPassword.waitForExistence(timeout: 30), "Cold restart did not retain account for unlock")
        capture(app, "cold-account-login")
        coldPassword.tap(); coldPassword.typeText("FE2-public-test-2026!\n")
        XCTAssertTrue(home.waitForExistence(timeout: 60), "First cold-start password submission did not reach home")
        capture(app, "cold-unlocked-home")
        let settings = app.buttons["设置"]
        XCTAssertTrue(settings.waitForExistence(timeout: 15)); XCTAssertTrue(settings.isHittable); settings.tap()
        capture(app, "settings-page")
        let appearance = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "主题与外观")).firstMatch
        XCTAssertTrue(appearance.waitForExistence(timeout: 15), "Appearance settings button missing")
        XCTAssertTrue(appearance.isHittable); appearance.tap()
        capture(app, "appearance-before")
        // Preserve the real reminder timing; collect theme frames after it naturally expires.
        Thread.sleep(forTimeInterval: 10)
        for (label, stage) in [("深色 ·", "fixed-dark-system-light"), ("浅色 ·", "fixed-light-system-light"), ("跟随系统", "system-mode-system-light")] {
            let control = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", label)).firstMatch
            XCTAssertTrue(control.waitForExistence(timeout: 15), "Theme label missing: " + label)
            XCTAssertTrue(control.isHittable); control.tap()
            // Real account persistence and theme coordinator run; no injected CSS or IPC.
            Thread.sleep(forTimeInterval: 1.5)
            capture(app, stage)
        }

    }
}
