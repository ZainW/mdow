import AppKit
import QuartzCore

/// `MDOW_ACTIONS="scrollbench=14"` scrolls the document down by the given number of points per
/// display frame, to the end or for 900 frames, and logs how long the frames took. Hitches are frames that took more than
/// one and a half refresh intervals.
@MainActor final class ScrollBench: NSObject {
  private static var current: ScrollBench?

  private let scrollView: NSScrollView
  private let step: CGFloat
  private var link: CADisplayLink?
  private var stamps: [CFTimeInterval] = []
  private var interval: CFTimeInterval = 1 / 60

  static func run(_ scrollView: NSScrollView, step: CGFloat) {
    let bench = ScrollBench(scrollView: scrollView, step: step)
    current = bench
    bench.start()
  }

  private init(scrollView: NSScrollView, step: CGFloat) {
    self.scrollView = scrollView
    self.step = step
  }

  private func start() {
    let clip = scrollView.contentView
    clip.scroll(to: NSPoint(x: 0, y: -scrollView.contentInsets.top))
    scrollView.reflectScrolledClipView(clip)
    let link = scrollView.displayLink(target: self, selector: #selector(tick(_:)))
    link.add(to: .main, forMode: .common)
    self.link = link
  }

  @objc private func tick(_ link: CADisplayLink) {
    stamps.append(CACurrentMediaTime())
    interval = link.targetTimestamp - link.timestamp
    let clip = scrollView.contentView
    let maxY = (scrollView.documentView?.frame.height ?? 0) - clip.bounds.height
    let next = min(clip.bounds.minY + step, maxY)
    clip.scroll(to: NSPoint(x: 0, y: next))
    scrollView.reflectScrolledClipView(clip)
    // About fifteen seconds at 60 Hz is plenty to see the pattern.
    if next >= maxY || stamps.count >= 900 { finish() }
  }

  private func finish() {
    link?.invalidate()
    link = nil
    let deltas = zip(stamps.dropFirst(), stamps).map { ($0 - $1) * 1000 }.sorted()
    guard !deltas.isEmpty else { return }
    let budget = interval * 1000
    func percentile(_ p: Double) -> Double { deltas[min(deltas.count - 1, Int(Double(deltas.count) * p))] }
    let hitches = deltas.filter { $0 > budget * 1.5 }
    let lost = hitches.reduce(0) { $0 + $1 - budget }
    let mean = deltas.reduce(0, +) / Double(deltas.count)
    NSLog(
      "MDOWBENCH frames=%d budget=%.1fms mean=%.2fms p50=%.2f p95=%.2f p99=%.2f max=%.2f hitches=%d lost=%.0fms",
      deltas.count, budget, mean, percentile(0.5), percentile(0.95), percentile(0.99), deltas.last!,
      hitches.count, lost)
    Self.current = nil
  }
}

/// `MDOW_ACTIONS="resizebench"` narrows the window and widens it again over 240 frames, like a
/// live resize, and logs the frame times the same way.
@MainActor final class ResizeBench: NSObject {
  private static var current: ResizeBench?

  private let window: NSWindow
  private let start: NSRect
  private var link: CADisplayLink?
  private var stamps: [CFTimeInterval] = []

  static func run(_ window: NSWindow) {
    let bench = ResizeBench(window: window)
    current = bench
    let link = window.contentView!.displayLink(target: bench, selector: #selector(tick(_:)))
    link.add(to: .main, forMode: .common)
    bench.link = link
  }

  private init(window: NSWindow) {
    self.window = window
    self.start = window.frame
  }

  @objc private func tick(_ link: CADisplayLink) {
    stamps.append(CACurrentMediaTime())
    let progress = Double(stamps.count) / 240
    let width = start.width - 420 * sin(progress * .pi)
    var frame = start
    frame.size.width = width.rounded()
    window.setFrame(frame, display: true)
    guard stamps.count >= 240 else { return }
    link.invalidate()
    window.setFrame(start, display: true)
    let deltas = zip(stamps.dropFirst(), stamps).map { ($0 - $1) * 1000 }.sorted()
    let budget = (link.targetTimestamp - link.timestamp) * 1000
    let hitches = deltas.filter { $0 > budget * 1.5 }
    NSLog(
      "MDOWBENCH resize frames=%d mean=%.2fms p50=%.2f p95=%.2f max=%.2f hitches=%d",
      deltas.count, deltas.reduce(0, +) / Double(deltas.count), deltas[deltas.count / 2],
      deltas[Int(Double(deltas.count) * 0.95)], deltas.last!, hitches.count)
    Self.current = nil
  }
}
