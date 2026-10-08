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
    func testCurrentProductionAccountCreateLockUnlock() {
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
        let loginPassword = field(app, "主密码")
        XCTAssertTrue(loginPassword.waitForExistence(timeout: 30), "Lock did not reach password login")
        capture(app, "current-locked-login")
        loginPassword.tap(); loginPassword.typeText("FE2-public-test-2026!\n")
        XCTAssertTrue(home.waitForExistence(timeout: 60), "First password submission did not return to home")
        XCTAssertTrue(home.isHittable)
        capture(app, "current-unlocked-home")
    }
}
