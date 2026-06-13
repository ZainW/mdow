import MdowNativeCore
import SwiftUI

struct SettingsView: View {
    @EnvironmentObject private var store: DocumentStore
    @Environment(\.dismiss) private var dismiss
    var showsHeader = true
    var onClose: (() -> Void)?

    var body: some View {
        VStack(spacing: 0) {
            if showsHeader {
                header
            }

            ScrollView {
                VStack(alignment: .leading, spacing: 26) {
                    typographyPreview

                    settingsSection("Appearance") {
                        settingsRow(
                            title: "Theme",
                            subtitle: "Follow the system appearance or pin the app to a theme."
                        ) {
                            Picker("Theme", selection: $store.appTheme) {
                                ForEach(AppTheme.allCases) { theme in
                                    Text(theme.title).tag(theme)
                                }
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                            .frame(width: 236)
                        }

                        sectionDivider

                        settingsRow(
                            title: "Interface scale",
                            subtitle: "Adjust density for controls, tabs, and sidebars."
                        ) {
                            Picker("Interface scale", selection: $store.interfaceScale) {
                                ForEach(InterfaceScale.allCases) { scale in
                                    Text(scale.title).tag(scale)
                                }
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                            .frame(width: 292)
                        }
                    }

                    settingsSection("Reader") {
                        settingsRow(
                            title: "Reading width",
                            subtitle: "Choose the Markdown column width for focused reading."
                        ) {
                            Picker("Reading width", selection: $store.readingWidth) {
                                ForEach(ReadingWidth.allCases) { width in
                                    Text(width.title).tag(width)
                                }
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                            .frame(width: 292)
                        }

                        sectionDivider

                        settingsRow(
                            title: "Reader zoom",
                            subtitle: "Scale rendered Markdown from 60% to 200%."
                        ) {
                            zoomControl
                        }

                        sectionDivider

                        settingsRow(
                            title: "Full width",
                            subtitle: "Let wide documents use the full reader area."
                        ) {
                            Toggle("", isOn: Binding(
                                get: { store.wideMode },
                                set: { _ in store.toggleWideMode() }
                            ))
                            .toggleStyle(.switch)
                            .labelsHidden()
                            .frame(width: 236, alignment: .trailing)
                        }
                    }

                    settingsSection("Typography") {
                        settingsRow(
                            title: "Content font",
                            subtitle: "Used by Markdown prose and headings."
                        ) {
                            Picker("Content font", selection: $store.contentFont) {
                                ForEach(ContentFontPreset.allCases) { font in
                                    Text(font.title).tag(font)
                                }
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                            .frame(width: 336)
                        }

                        sectionDivider

                        settingsRow(
                            title: "Code font",
                            subtitle: "Used by fenced code blocks and inline code."
                        ) {
                            Picker("Code font", selection: $store.codeFont) {
                                ForEach(CodeFontPreset.allCases) { font in
                                    Text(font.title).tag(font)
                                }
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                            .frame(width: 336)
                        }
                    }

                    settingsSection("Navigation") {
                        settingsRow(
                            title: "Default sidebar",
                            subtitle: "Choose the active sidebar surface."
                        ) {
                            Picker("Sidebar", selection: $store.sidebarMode) {
                                ForEach(SidebarMode.allCases) { mode in
                                    Text(mode.title).tag(mode)
                                }
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                            .frame(width: 236)
                        }
                    }

                    nativeRendererNote

                    HStack {
                        Button("Reset to defaults") {
                            resetDefaults()
                        }
                        .buttonStyle(.bordered)

                        Spacer()
                    }
                    .padding(.top, -4)
                }
                .frame(maxWidth: 760, alignment: .leading)
                .padding(.horizontal, 42)
                .padding(.top, 30)
                .padding(.bottom, 48)
                .frame(maxWidth: .infinity, alignment: .center)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(MdowStyle.background)
    }

    private var header: some View {
        HStack(spacing: 12) {
            Button {
                close()
            } label: {
                Image(systemName: "chevron.left")
                    .font(.system(size: 13, weight: .semibold))
                    .frame(width: 28, height: 24)
            }
            .buttonStyle(ChromeIconButtonStyle())
            .help("Back to document")
            .accessibilityLabel("Back to document")

            VStack(alignment: .leading, spacing: 2) {
                Text("Settings")
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(MdowStyle.foreground)
                Text("Tune how Markdown reads.")
                    .font(.system(size: 12))
                    .foregroundStyle(MdowStyle.mutedForeground)
            }

            Spacer()
        }
        .padding(.leading, 14)
        .padding(.trailing, 16)
        .frame(height: 58)
        .background(MdowStyle.background)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(MdowStyle.borderSubtle.opacity(0.32))
                .frame(height: 1)
        }
    }

    private var typographyPreview: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Text("Preview")
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundStyle(MdowStyle.mutedForeground)
                    .textCase(.uppercase)
                    .tracking(1.2)
                Spacer()
                Text("\(store.zoomLevel)%")
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(MdowStyle.mutedForeground)
                    .monospacedDigit()
            }

            VStack(alignment: .leading, spacing: 7) {
                Text("The quiet morning")
                    .font(store.contentFont.font(size: previewScale(22), weight: .semibold))
                    .foregroundStyle(MdowStyle.foreground)
                HStack(alignment: .firstTextBaseline, spacing: 4) {
                    Text("Words settle into rhythm, and")
                        .font(store.contentFont.font(size: previewScale(14.5)))
                    Text("inline code")
                        .font(store.codeFont.font(size: previewScale(13.5)))
                        .padding(.horizontal, 5)
                        .padding(.vertical, 2)
                        .background(MdowStyle.muted, in: RoundedRectangle(cornerRadius: 4))
                    Text("does too.")
                        .font(store.contentFont.font(size: previewScale(14.5)))
                }
                .foregroundStyle(MdowStyle.foreground.opacity(0.86))
            }
            .padding(18)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(MdowStyle.elevatedSurface, in: RoundedRectangle(cornerRadius: 8))
            .overlay {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(MdowStyle.borderSubtle.opacity(0.56), lineWidth: 1)
            }
        }
    }

    private var zoomControl: some View {
        HStack(spacing: 8) {
            Button {
                store.zoomOut()
            } label: {
                Image(systemName: "minus")
                    .frame(width: 24, height: 22)
            }
            .disabled(store.zoomLevel <= 60)
            .help("Zoom out")
            .accessibilityLabel("Zoom out")

            Text("\(store.zoomLevel)%")
                .font(.system(size: 12, design: .monospaced))
                .monospacedDigit()
                .frame(width: 48)

            Button {
                store.zoomIn()
            } label: {
                Image(systemName: "plus")
                    .frame(width: 24, height: 22)
            }
            .disabled(store.zoomLevel >= 200)
            .help("Zoom in")
            .accessibilityLabel("Zoom in")

            Button("Reset") {
                store.resetZoom()
            }
            .disabled(store.zoomLevel == 100)
            .help("Reset zoom")
        }
        .buttonStyle(.bordered)
        .frame(width: 236, alignment: .trailing)
    }

    private var nativeRendererNote: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: "checkmark.shield")
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(MdowStyle.primary)
                .frame(width: 22, height: 22)

            VStack(alignment: .leading, spacing: 3) {
                Text("Native renderer")
                    .font(.system(size: 13, weight: .medium))
                Text("This build uses SwiftUI and AppKit rendering without embedding a browser view.")
                    .font(.system(size: 12))
                    .foregroundStyle(MdowStyle.mutedForeground)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 13)
        .background(MdowStyle.muted.opacity(0.30), in: RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(MdowStyle.borderSubtle.opacity(0.48), lineWidth: 1)
        }
    }

    private var sectionDivider: some View {
        Rectangle()
            .fill(MdowStyle.borderSubtle.opacity(0.40))
            .frame(height: 1)
            .padding(.leading, 18)
    }

    private func settingsSection<Content: View>(
        _ title: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundStyle(MdowStyle.mutedForeground)
                .textCase(.uppercase)
                .tracking(1.2)
                .padding(.leading, 2)

            VStack(spacing: 0) {
                content()
            }
            .background(MdowStyle.elevatedSurface, in: RoundedRectangle(cornerRadius: 8))
            .overlay {
                RoundedRectangle(cornerRadius: 8)
                    .stroke(MdowStyle.borderSubtle.opacity(0.56), lineWidth: 1)
            }
        }
    }

    private func settingsRow<Accessory: View>(
        title: String,
        subtitle: String,
        @ViewBuilder accessory: () -> Accessory
    ) -> some View {
        HStack(alignment: .center, spacing: 24) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(MdowStyle.foreground)
                Text(subtitle)
                    .font(.system(size: 12))
                    .foregroundStyle(MdowStyle.mutedForeground)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            accessory()
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 14)
        .frame(minHeight: 68)
    }

    private func previewScale(_ value: CGFloat) -> CGFloat {
        value * CGFloat(store.zoomLevel) / 100
    }

    private func resetDefaults() {
        store.appTheme = .system
        store.contentFont = .inter
        store.codeFont = .geistMono
        store.interfaceScale = .compact
        store.readingWidth = .standard
        store.sidebarMode = .recents
        store.resetZoom()
        if store.wideMode {
            store.toggleWideMode()
        }
    }

    private func close() {
        if let onClose {
            onClose()
        } else {
            dismiss()
        }
    }
}
