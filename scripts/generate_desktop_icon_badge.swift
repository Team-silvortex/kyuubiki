#!/usr/bin/env swift

import AppKit
import Foundation

struct Variant {
    let label: String
    let color: NSColor
}

func parseHexColor(_ value: String) -> NSColor? {
    let hex = value.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
    guard hex.count == 6 || hex.count == 8 else { return nil }

    var int: UInt64 = 0
    guard Scanner(string: hex).scanHexInt64(&int) else { return nil }

    let a, r, g, b: UInt64
    if hex.count == 8 {
        a = (int >> 24) & 0xff
        r = (int >> 16) & 0xff
        g = (int >> 8) & 0xff
        b = int & 0xff
    } else {
        a = 0xff
        r = (int >> 16) & 0xff
        g = (int >> 8) & 0xff
        b = int & 0xff
    }

    return NSColor(
        calibratedRed: CGFloat(r) / 255,
        green: CGFloat(g) / 255,
        blue: CGFloat(b) / 255,
        alpha: CGFloat(a) / 255
    )
}

func usage() -> Never {
    fputs("usage: generate_desktop_icon_badge.swift <base-png> <output-png> <label> <badge-hex>\n", stderr)
    exit(1)
}

guard CommandLine.arguments.count == 5 else {
    usage()
}

let basePath = CommandLine.arguments[1]
let outputPath = CommandLine.arguments[2]
let label = CommandLine.arguments[3]
let badgeHex = CommandLine.arguments[4]

guard let baseImage = NSImage(contentsOfFile: basePath) else {
    fputs("failed to open base image at \(basePath)\n", stderr)
    exit(1)
}

guard let badgeColor = parseHexColor(badgeHex) else {
    fputs("failed to parse badge color \(badgeHex)\n", stderr)
    exit(1)
}

let canvasSize = NSSize(width: 1024, height: 1024)
let canvasRect = NSRect(origin: .zero, size: canvasSize)
let badgeSize: CGFloat = 332
let badgeMargin: CGFloat = 54
let badgeRect = NSRect(
    x: canvasSize.width - badgeSize - badgeMargin,
    y: badgeMargin,
    width: badgeSize,
    height: badgeSize
)

guard let bitmap = NSBitmapImageRep(
    bitmapDataPlanes: nil,
    pixelsWide: Int(canvasSize.width),
    pixelsHigh: Int(canvasSize.height),
    bitsPerSample: 8,
    samplesPerPixel: 4,
    hasAlpha: true,
    isPlanar: false,
    colorSpaceName: .deviceRGB,
    bytesPerRow: 0,
    bitsPerPixel: 0
) else {
    fputs("failed to create bitmap canvas\n", stderr)
    exit(1)
}

bitmap.size = canvasSize

guard let context = NSGraphicsContext(bitmapImageRep: bitmap) else {
    fputs("failed to create graphics context\n", stderr)
    exit(1)
}

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = context
context.imageInterpolation = .high

baseImage.draw(in: canvasRect, from: .zero, operation: .copy, fraction: 1.0)

let shadow = NSShadow()
shadow.shadowColor = NSColor(calibratedWhite: 0.04, alpha: 0.34)
shadow.shadowBlurRadius = 20
shadow.shadowOffset = NSSize(width: 0, height: -8)
shadow.set()

let badgePath = NSBezierPath(roundedRect: badgeRect, xRadius: 96, yRadius: 96)
badgeColor.setFill()
badgePath.fill()

NSColor.white.withAlphaComponent(0.94).setStroke()
badgePath.lineWidth = 18
badgePath.stroke()

let paragraph = NSMutableParagraphStyle()
paragraph.alignment = .center

let font = NSFont.systemFont(ofSize: 188, weight: .black)
let attributes: [NSAttributedString.Key: Any] = [
    .font: font,
    .foregroundColor: NSColor.white,
    .paragraphStyle: paragraph
]

let textRect = NSRect(
    x: badgeRect.origin.x,
    y: badgeRect.origin.y + 58,
    width: badgeRect.width,
    height: badgeRect.height - 88
)

label.draw(in: textRect, withAttributes: attributes)

NSGraphicsContext.restoreGraphicsState()

guard let png = bitmap.representation(using: .png, properties: [:]) else {
    fputs("failed to encode output png\n", stderr)
    exit(1)
}

do {
    try png.write(to: URL(fileURLWithPath: outputPath))
} catch {
    fputs("failed to write output png: \(error)\n", stderr)
    exit(1)
}
