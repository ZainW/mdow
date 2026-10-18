import Foundation
import QuartzCore

/// `MDOW_PERF=1` logs how long the expensive steps take (rendering, layout, assets), for
/// checking performance without Instruments.
public enum PerfLog {
  public static let enabled = ProcessInfo.processInfo.environment["MDOW_PERF"] != nil

  @discardableResult
  public static func measure<T>(_ label: @autoclosure () -> String, _ body: () throws -> T) rethrows -> T {
    guard enabled else { return try body() }
    let start = CACurrentMediaTime()
    defer { NSLog("MDOWPERF %@ %.1fms", label(), (CACurrentMediaTime() - start) * 1000) }
    return try body()
  }

  public static func note(_ message: @autoclosure () -> String) {
    if enabled { NSLog("MDOWPERF %@", message()) }
  }

  /// Logs how long after the process started this point was reached.
  public static func milestone(_ name: @autoclosure () -> String) {
    guard enabled else { return }
    var info = kinfo_proc()
    var size = MemoryLayout<kinfo_proc>.stride
    var mib: [Int32] = [CTL_KERN, KERN_PROC, KERN_PROC_PID, getpid()]
    guard sysctl(&mib, 4, &info, &size, nil, 0) == 0 else { return }
    let started = info.kp_proc.p_starttime
    let start = Double(started.tv_sec) + Double(started.tv_usec) / 1_000_000
    NSLog("MDOWPERF %@ at %.0fms", name(), (Date().timeIntervalSince1970 - start) * 1000)
  }
}
