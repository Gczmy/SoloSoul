import UIKit
@main final class App: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    func application(_ application: UIApplication, didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // 测试助手启动后以自身沙箱 API 写入公开文件，供真实系统选择器读取。
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        do {
            try FileManager.default.createDirectory(at: docs, withIntermediateDirectories: true)
            for filename in ["FE2-public-note.txt", "FE2-public-photo.png", "FE2-public-second.png"] {
                let resourceName = filename.hasSuffix(".png") ? filename.replacingOccurrences(of: ".png", with: ".data") : filename
                guard let resource = Bundle.main.url(forResource: resourceName, withExtension: nil) else { fatalError("Missing public fixture") }
                let data = try Data(contentsOf: resource)
                try data.write(to: docs.appendingPathComponent(filename), options: .atomic)
            }
        } catch { fatalError("Public fixture provider preparation failed: \(error)") }
        let window = UIWindow(frame: UIScreen.main.bounds)
        window.rootViewController = UIViewController()
        window.makeKeyAndVisible()
        self.window = window
        return true
    }
}
