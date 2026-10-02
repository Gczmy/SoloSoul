// RF-1069：独立 UIKit/WKWebView 对照，仅合成中文，不使用 SoloSoul 账户或网络。
import UIKit
import WebKit

final class ProbeController: UIViewController, WKNavigationDelegate {
    let web = WKWebView()
    var native: [[String: Any]] = []
    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .white
        let stack = UIStackView()
        stack.axis = .vertical
        stack.spacing = 8
        stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)
        let examples: [(String, UIFont)] = [
            ("UIKit system", .systemFont(ofSize: 19)),
            ("UIKit PingFang SC", UIFont(name: "PingFangSC-Regular", size: 19) ?? .systemFont(ofSize: 19)),
        ]
        for (name, font) in examples {
            let label = UILabel()
            label.numberOfLines = 0
            label.text = "\(name): 欢迎来到 独灵 新建账户"
            label.font = font
            label.textColor = .black
            stack.addArrangedSubview(label)
            native.append(["label": name, "font": font.fontName, "text": label.text!])
        }
        web.translatesAutoresizingMaskIntoConstraints = false
        web.navigationDelegate = self
        view.addSubview(web)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor, constant: 8),
            stack.leadingAnchor.constraint(equalTo: view.leadingAnchor, constant: 14),
            stack.trailingAnchor.constraint(equalTo: view.trailingAnchor, constant: -14),
            web.topAnchor.constraint(equalTo: stack.bottomAnchor, constant: 14),
            web.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            web.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            web.bottomAnchor.constraint(equalTo: view.safeAreaLayoutGuide.bottomAnchor),
        ])
        web.loadHTMLString("""
        <!doctype html><html lang="zh-CN"><meta charset="UTF-8">
        <meta name="viewport" content="width=device-width,initial-scale=1">
        <style>body{color:#111;background:white;margin:14px;font-size:19px}p{margin:18px 0}.name{font-family:Arial;font-size:14px;color:#555}</style>
        <div class="name">WKWebView default</div><p class="sample">欢迎来到 独灵 新建账户</p>
        <div class="name">WKWebView -apple-system</div><p class="sample" style="font-family:-apple-system">欢迎来到 独灵 新建账户</p>
        <div class="name">WKWebView SoloSoul stack</div><p class="sample" style="font-family:-apple-system,BlinkMacSystemFont,'SF Pro Text','Segoe UI','Noto Sans SC','PingFang SC','Hiragino Sans GB','Microsoft YaHei',sans-serif">欢迎来到 独灵 新建账户</p>
        <div class="name">WKWebView explicit PingFang SC</div><p class="sample" style="font-family:'PingFang SC'">欢迎来到 独灵 新建账户</p>
        <div class="name">WKWebView sans-serif</div><p class="sample" style="font-family:sans-serif">欢迎来到 独灵 新建账户</p>
        <div class="name">WKWebView system-ui</div><p class="sample" style="font-family:system-ui">欢迎来到 独灵 新建账户</p>
        </html>
        """, baseURL: nil)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            webView.evaluateJavaScript("JSON.stringify({encoding:document.characterSet,userAgent:navigator.userAgent,samples:[...document.querySelectorAll('.sample')].map(e=>({text:e.textContent,codepoints:[...e.textContent].map(c=>c.codePointAt(0)),font:getComputedStyle(e).fontFamily,weight:getComputedStyle(e).fontWeight}))})") { value, error in
                let record: [String: Any] = ["native": self.native, "web": value ?? NSNull(), "error": error?.localizedDescription ?? NSNull()]
                let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
                try? JSONSerialization.data(withJSONObject: record, options: [.prettyPrinted, .sortedKeys]).write(to: directory.appendingPathComponent("probe.json"))
                webView.takeSnapshot(with: nil) { image, snapshotError in
                    try? image?.pngData()?.write(to: directory.appendingPathComponent("webview.png"))
                    if let snapshotError { try? snapshotError.localizedDescription.write(to: directory.appendingPathComponent("snapshot-error.txt"), atomically: true, encoding: .utf8) }
                }
            }
        }
    }
}

@main
final class ProbeApp: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        let window = UIWindow(frame: UIScreen.main.bounds)
        window.rootViewController = ProbeController()
        window.makeKeyAndVisible()
        self.window = window
        return true
    }
}
