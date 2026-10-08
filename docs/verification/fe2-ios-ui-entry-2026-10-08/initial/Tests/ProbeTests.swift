import XCTest
final class ProbeTests: XCTestCase {
    func testNativeOnboardingEntry() {
        continueAfterFailure = false
        let app = XCUIApplication(bundleIdentifier: "com.solosoul.app")
        app.launch()
        let next = app.buttons["下一步"]
        XCTAssertTrue(next.waitForExistence(timeout: 30), "Production welcome action missing")
        XCTAssertTrue(next.isHittable)
        let before = XCTAttachment(screenshot: app.screenshot())
        before.name = "production-welcome"
        before.lifetime = .keepAlways
        add(before)
        next.tap()
        let back = app.buttons["返回"]
        XCTAssertTrue(back.waitForExistence(timeout: 15), "Production next step missing")
        let after = XCTAttachment(screenshot: app.screenshot())
        after.name = "production-after-next"
        after.lifetime = .keepAlways
        add(after)
        let hierarchy = XCTAttachment(string: app.debugDescription)
        hierarchy.name = "production-after-next-hierarchy"
        hierarchy.lifetime = .keepAlways
        add(hierarchy)
        app.terminate()
    }
}
