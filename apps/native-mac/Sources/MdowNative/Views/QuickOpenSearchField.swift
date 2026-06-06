import AppKit
import SwiftUI

struct QuickOpenSearchField: NSViewRepresentable {
    @Binding var text: String
    @Binding var isFocused: Bool

    let placeholder: String
    let onSubmit: () -> Void
    let onMoveUp: () -> Void
    let onMoveDown: () -> Void
    let onClose: () -> Void

    func makeNSView(context: Context) -> QuickOpenNSTextField {
        let textField = QuickOpenNSTextField()
        textField.delegate = context.coordinator
        textField.placeholderString = placeholder
        textField.isBezeled = false
        textField.drawsBackground = false
        textField.focusRingType = .none
        textField.font = .systemFont(ofSize: 15)
        textField.textColor = .labelColor
        textField.setAccessibilityLabel("Quick Open")
        textField.setAccessibilityRole(.textField)
        context.coordinator.configure(
            onSubmit: onSubmit,
            onMoveUp: onMoveUp,
            onMoveDown: onMoveDown,
            onClose: onClose
        )
        textField.onSubmit = context.coordinator.submit
        textField.onMoveUp = context.coordinator.moveUp
        textField.onMoveDown = context.coordinator.moveDown
        textField.onClose = context.coordinator.close
        return textField
    }

    func updateNSView(_ textField: QuickOpenNSTextField, context: Context) {
        if textField.stringValue != text {
            textField.stringValue = text
        }

        textField.placeholderString = placeholder
        context.coordinator.configure(
            onSubmit: onSubmit,
            onMoveUp: onMoveUp,
            onMoveDown: onMoveDown,
            onClose: onClose
        )

        guard isFocused else { return }
        DispatchQueue.main.async {
            guard let window = textField.window else { return }
            if textField.currentEditor() != nil { return }
            window.makeFirstResponder(textField)
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text, isFocused: $isFocused)
    }

    final class Coordinator: NSObject, NSTextFieldDelegate {
        private let text: Binding<String>
        private let isFocused: Binding<Bool>
        private var onSubmit: (() -> Void)?
        private var onMoveUp: (() -> Void)?
        private var onMoveDown: (() -> Void)?
        private var onClose: (() -> Void)?

        init(text: Binding<String>, isFocused: Binding<Bool>) {
            self.text = text
            self.isFocused = isFocused
        }

        func configure(
            onSubmit: @escaping () -> Void,
            onMoveUp: @escaping () -> Void,
            onMoveDown: @escaping () -> Void,
            onClose: @escaping () -> Void
        ) {
            self.onSubmit = onSubmit
            self.onMoveUp = onMoveUp
            self.onMoveDown = onMoveDown
            self.onClose = onClose
        }

        func controlTextDidChange(_ notification: Notification) {
            guard let textField = notification.object as? NSTextField else { return }
            text.wrappedValue = textField.stringValue
        }

        func controlTextDidBeginEditing(_ notification: Notification) {
            isFocused.wrappedValue = true
        }

        func controlTextDidEndEditing(_ notification: Notification) {
            isFocused.wrappedValue = false
        }

        func control(
            _ control: NSControl,
            textView: NSTextView,
            doCommandBy commandSelector: Selector
        ) -> Bool {
            switch commandSelector {
            case #selector(NSResponder.insertNewline(_:)),
                 #selector(NSResponder.insertNewlineIgnoringFieldEditor(_:)):
                submit()
                return true
            case #selector(NSResponder.moveUp(_:)):
                moveUp()
                return true
            case #selector(NSResponder.moveDown(_:)):
                moveDown()
                return true
            case #selector(NSResponder.cancelOperation(_:)):
                close()
                return true
            default:
                return false
            }
        }

        func submit() {
            onSubmit?()
        }

        func moveUp() {
            onMoveUp?()
        }

        func moveDown() {
            onMoveDown?()
        }

        func close() {
            onClose?()
        }
    }
}

final class QuickOpenNSTextField: NSTextField {
    var onSubmit: (() -> Void)?
    var onMoveUp: (() -> Void)?
    var onMoveDown: (() -> Void)?
    var onClose: (() -> Void)?

    override func keyDown(with event: NSEvent) {
        switch event.keyCode {
        case 36, 76:
            onSubmit?()
        case 125:
            onMoveDown?()
        case 126:
            onMoveUp?()
        case 53:
            onClose?()
        default:
            super.keyDown(with: event)
        }
    }
}
