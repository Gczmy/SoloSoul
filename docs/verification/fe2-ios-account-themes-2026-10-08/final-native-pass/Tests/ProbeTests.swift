import XCTest
final class ProbeTests: XCTestCase {
    private let password = "FE2-public-test-2026!"
    private let a = "FE2 Theme A"
    private let b = "FE2 Theme B"
    private func capture(_ app: XCUIApplication, _ name: String) {
        let image = XCTAttachment(screenshot: app.screenshot()); image.name = name; image.lifetime = .keepAlways; add(image)
        let tree = XCTAttachment(string: app.debugDescription); tree.name = name + "-hierarchy"; tree.lifetime = .keepAlways; add(tree)
    }
    private func field(_ app: XCUIApplication, _ name: String) -> XCUIElement {
        let secure = app.secureTextFields[name]; return secure.exists ? secure : app.textFields[name]
    }
    private func register(_ app: XCUIApplication, _ name: String) {
        let entry = field(app, "账户名称"); XCTAssertTrue(entry.waitForExistence(timeout: 30)); entry.tap(); entry.typeText(name)
        let primary = field(app, "主密码"); XCTAssertTrue(primary.exists); primary.tap(); primary.typeText(password)
        let confirm = field(app, "确认密码"); XCTAssertTrue(confirm.exists); confirm.tap(); confirm.typeText(password + "\n")
        XCTAssertTrue(app.buttons["首页"].waitForExistence(timeout: 60)); capture(app, name + "-created-home")
        Thread.sleep(forTimeInterval: 10) // 保留正式提醒，等待其自然退出。
    }
    private func settings(_ app: XCUIApplication) {
        let button = app.buttons["设置"]; XCTAssertTrue(button.waitForExistence(timeout: 15)); XCTAssertTrue(button.isHittable); button.tap()
    }
    private func account(_ app: XCUIApplication, _ expected: String, _ stage: String) {
        settings(app)
        let button = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "账户管理")).firstMatch
        XCTAssertTrue(button.waitForExistence(timeout: 15)); XCTAssertTrue(button.isHittable); button.tap()
        let name = app.textFields["账户名"]; XCTAssertTrue(name.waitForExistence(timeout: 15)); XCTAssertEqual(name.value as? String, expected)
        capture(app, stage + "-identity")
        app.buttons["返回"].tap()
    }
    private func appearance(_ app: XCUIApplication) {
        let button = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", "主题与外观")).firstMatch
        XCTAssertTrue(button.waitForExistence(timeout: 15)); XCTAssertTrue(button.isHittable); button.tap()
    }
    private func selectTheme(_ app: XCUIApplication, _ dark: Bool, _ accent: String, _ stage: String) {
        settings(app); appearance(app)
        let mode = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", dark ? "深色 ·" : "浅色 ·")).firstMatch
        XCTAssertTrue(mode.waitForExistence(timeout: 15)); XCTAssertTrue(mode.isHittable); mode.tap()
        let color = app.buttons[accent]; XCTAssertTrue(color.exists); XCTAssertTrue(color.isHittable); color.tap()
        Thread.sleep(forTimeInterval: 1.5); capture(app, stage)
    }
    private func lock(_ app: XCUIApplication, _ stage: String) {
        app.buttons["首页"].tap(); let button = app.buttons["锁定保险库"]
        XCTAssertTrue(button.waitForExistence(timeout: 15)); XCTAssertTrue(button.isHittable); button.tap()
        XCTAssertTrue(field(app, "输入密码").waitForExistence(timeout: 30)); capture(app, stage)
    }
    private func unlock(_ app: XCUIApplication, _ stage: String) {
        let entry = field(app, "输入密码"); XCTAssertTrue(entry.waitForExistence(timeout: 20)); entry.tap(); entry.typeText(password + "\n")
        XCTAssertTrue(app.buttons["首页"].waitForExistence(timeout: 60), "First real submission must unlock selected account")
        capture(app, stage)
    }
    private func choose(_ app: XCUIApplication, _ from: String, _ to: String, _ stage: String) {
        capture(app, stage + "-before-account-picker")
        let candidates = app.descendants(matching: .any).matching(NSPredicate(format: "label BEGINSWITH %@ OR value BEGINSWITH %@", from, from)).allElementsBoundByIndex.filter {
            $0.isHittable && $0.frame.width > 100 && $0.frame.height > 15 && $0.frame.height < 80 && $0.elementType != .staticText
        }
        XCTAssertEqual(candidates.count, 1, "Native account select must be uniquely identified from its current value")
        candidates[0].tap(); capture(app, stage + "-native-account-picker-open")
        // 首次真实层级证明 iOS 使用 CollectionView 的 Button 选项，86pt 高。
        let options = app.buttons.matching(NSPredicate(format: "label BEGINSWITH %@", to))
        XCTAssertTrue(options.firstMatch.waitForExistence(timeout: 10)); XCTAssertEqual(options.count, 1)
        XCTAssertTrue(options.firstMatch.isHittable); options.firstMatch.tap()
        capture(app, stage + "-selected-account-login")
        XCTAssertTrue(app.descendants(matching: .any).matching(NSPredicate(format: "label BEGINSWITH %@ OR value BEGINSWITH %@", to, to)).firstMatch.exists, "Login select must report target account")
    }
    func testCurrentProductionTwoAccountThemeOwnership() {
        continueAfterFailure = false
        let app = XCUIApplication(bundleIdentifier: "com.solosoul.app"); app.launch()
        defer { capture(app, "final-state"); app.terminate() }
        for _ in 0..<4 { let next = app.buttons["下一步"]; XCTAssertTrue(next.waitForExistence(timeout: 30)); next.tap() }
        let done = app.buttons["完成"]; XCTAssertTrue(done.waitForExistence(timeout: 15)); done.tap()
        let create = app.buttons["不，创建新账户"]; XCTAssertTrue(create.waitForExistence(timeout: 30)); create.tap()
        register(app, a); account(app, a, "a-created"); app.buttons["首页"].tap()
        selectTheme(app, false, "海洋蓝", "a-light-ocean-selected"); lock(app, "a-locked-login")
        let another = app.buttons["创建新账户"]; XCTAssertTrue(another.exists); XCTAssertTrue(another.isHittable); another.tap()
        register(app, b); account(app, b, "b-created"); app.buttons["首页"].tap()
        selectTheme(app, true, "玫瑰红", "b-dark-rose-selected"); lock(app, "b-locked-login")
        choose(app, b, a, "switch-to-a"); unlock(app, "a-unlocked-home")
        account(app, a, "a-restored"); appearance(app); capture(app, "a-restored-light-ocean")
        lock(app, "a-relocked-login"); choose(app, a, b, "switch-to-b"); unlock(app, "b-unlocked-home")
        account(app, b, "b-restored"); appearance(app); capture(app, "b-restored-dark-rose")
    }
}
