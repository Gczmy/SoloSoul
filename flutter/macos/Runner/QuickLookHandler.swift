import Cocoa
import FlutterMacOS
import Quartz

// =============================================================================
// QuickLook Handler for PPTX/PPT Multi-page Preview
// =============================================================================

/// Manages macOS QLPreviewPanel for in-app PPTX/PPT preview.
/// Communicates with Flutter via MethodChannel for show/close events.
class QuickLookHandler: NSObject, QLPreviewPanelDataSource, QLPreviewPanelDelegate {
    static let shared = QuickLookHandler()

    private var currentFileURL: URL?
    private var channel: FlutterMethodChannel?

    /// Register the MethodChannel with the given Flutter engine.
    func register(with engine: FlutterEngine) {
        channel = FlutterMethodChannel(
            name: "solosoul/quicklook",
            binaryMessenger: engine.binaryMessenger
        )
        channel?.setMethodCallHandler { [weak self] (call, result) in
            switch call.method {
            case "showQuickLook":
                if let args = call.arguments as? [String: Any],
                   let filePath = args["filePath"] as? String {
                    self?.showQuickLook(filePath: filePath)
                    result(true)
                } else {
                    result(FlutterError(code: "INVALID_ARGS",
                                        message: "Missing filePath",
                                        details: nil))
                }
            default:
                result(FlutterMethodNotImplemented)
            }
        }
    }

    /// Show QLPreviewPanel for the given file path.
    func showQuickLook(filePath: String) {
        currentFileURL = URL(fileURLWithPath: filePath)

        guard let panel = QLPreviewPanel.shared() else {
            print("[QuickLook] ERROR: QLPreviewPanel.shared() returned nil")
            return
        }

        panel.dataSource = self
        panel.delegate = self
        panel.reloadData()
        panel.makeKeyAndOrderFront(nil)
    }

    // MARK: - QLPreviewPanelDataSource

    func numberOfPreviewItems(in panel: QLPreviewPanel) -> Int {
        return currentFileURL != nil ? 1 : 0
    }

    func previewPanel(_ panel: QLPreviewPanel, previewItemAt index: Int) -> QLPreviewItem {
        return currentFileURL! as QLPreviewItem
    }

    // MARK: - QLPreviewPanelDelegate

    func previewPanelDidClose(_ panel: QLPreviewPanel) {
        // Notify Dart that the panel is closed so it can clean up temp files
        if let path = currentFileURL?.path {
            channel?.invokeMethod("onQuickLookClosed", arguments: path)
        }
        currentFileURL = nil
    }
}
