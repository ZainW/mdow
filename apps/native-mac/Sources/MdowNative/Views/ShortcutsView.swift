import MdowNativeCore
import SwiftUI

struct ShortcutsView: View {
    @Binding var isPresented: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Keyboard Shortcuts")
                    .font(.system(size: 18, weight: .semibold))
                Spacer()
                Button {
                    isPresented = false
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 13, weight: .medium))
                        .frame(width: 28, height: 24)
                }
                .buttonStyle(ChromeIconButtonStyle())
                .help("Close shortcuts")
                .accessibilityLabel("Close shortcuts")
            }
            .padding(18)

            Rectangle()
                .fill(MdowStyle.borderSubtle)
                .frame(height: 1)

            VStack(alignment: .leading, spacing: 0) {
                ForEach(Array(KeyboardShortcutReference.groups.enumerated()), id: \.element.heading) { index, group in
                    if index > 0 {
                        Rectangle()
                            .fill(MdowStyle.borderSubtle)
                            .frame(height: 1)
                            .padding(.vertical, 6)
                    }

                    Text(group.heading.uppercased())
                        .font(.system(size: 10, weight: .medium, design: .monospaced))
                        .tracking(1.5)
                        .foregroundStyle(MdowStyle.mutedForeground.opacity(0.7))
                        .padding(.bottom, 4)

                    ForEach(group.items, id: \.label) { item in
                        HStack(spacing: 18) {
                            Text(item.label)
                                .font(.system(size: 13))
                                .foregroundStyle(MdowStyle.mutedForeground)
                            Spacer()
                            Text(item.keys)
                                .font(.system(size: 12, weight: .medium, design: .monospaced))
                                .foregroundStyle(MdowStyle.foreground)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(MdowStyle.muted, in: RoundedRectangle(cornerRadius: 5))
                                .overlay {
                                    RoundedRectangle(cornerRadius: 5)
                                        .stroke(MdowStyle.borderSubtle, lineWidth: 1)
                                }
                        }
                        .padding(.vertical, 4)
                    }
                }
            }
            .padding(18)
        }
        .frame(width: 420)
        .background(MdowStyle.background)
        .onExitCommand {
            isPresented = false
        }
        .accessibilityLabel("Keyboard Shortcuts")
    }
}
