import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    // Set Flutter view background to white to avoid black screen during startup
    flutterViewController.view.wantsLayer = true
    flutterViewController.view.layer?.backgroundColor = NSColor.white.cgColor

    // Set minimum window size for desktop security
    self.minSize = NSSize(width: 800, height: 600)

    // Persist window position and size across launches
    setFrameAutosaveName("MainFlutterWindow")

    RegisterGeneratedPlugins(registry: flutterViewController)

    super.awakeFromNib()
  }
}
