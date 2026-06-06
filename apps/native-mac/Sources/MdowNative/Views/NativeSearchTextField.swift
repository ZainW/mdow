import AppKit
import SwiftUI

struct NativeSearchTextField: NSViewRepresentable {
    @Binding var text: String
    @Binding var isFocused: Bool

    let onNext: () -> Void
    let onPrevious: () -> Void
    let onClose: () -> Void

    func makeNSView(context: Context) -> SearchNSTextField {
        let textField = SearchNSTextField()
        textField.delegate = context.coordinator
        textField.target = context.coordinator
        textField.action = #selector(Coordinator.submitFromAction(_:))
        textField.placeholderString = "Find in document"
        textField.isBezeled = false
        textField.drawsBackground = false
        textField.focusRingType = .none
        textField.font = .systemFont(ofSize: 12)
        textField.textColor = .labelColor
        textField.setAccessibilityLabel("Find in document")
        textField.setAccessibilityRole(.textField)
        context.coordinator.onNext = onNext
        context.coordinator.onPrevious = onPrevious
        context.coordinator.onClose = onClose
        textField.onSubmit = context.coordinator.submit(isShiftPressed:)
        textField.onClose = context.coordinator.close
        return textField
    }

    func updateNSView(_ textField: SearchNSTextField, context: Context) {
        if textField.stringValue != text {
            textField.stringValue = text
        }

        context.coordinator.onNext = onNext
        context.coordinator.onPrevious = onPrevious
        context.coordinator.onClose = onClose
        textField.onSubmit = context.coordinator.submit(isShiftPressed:)
        textField.onClose = context.coordinator.close

        guard isFocused else { return }
        DispatchQueue.main.async {
            guard let window = textField.window else { return }
            if textField.currentEditor() != nil { return }
            window.makeFirstResponder(textField)
            textField.currentEditor()?.selectAll(nil)
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text, isFocused: $isFocused)
    }

    final class Coordinator: NSObject, NSTextFieldDelegate {
        private let text: Binding<String>
        private let isFocused: Binding<Bool>
        var onNext: (() -> Void)?
        var onPrevious: (() -> Void)?
        var onClose: (() -> Void)?

        init(text: Binding<String>, isFocused: Binding<Bool>) {
            self.text = text
            self.isFocused = isFocused
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
                submit(isShiftPressed: NSApp.currentEvent?.modifierFlags.contains(.shift) == true)
                return true
            case #selector(NSResponder.cancelOperation(_:)):
                close()
                return true
            default:
                return false
            }
        }

        func submit(isShiftPressed: Bool) {
            isShiftPressed ? onPrevious?() : onNext?()
        }

        func close() {
            onClose?()
        }

        @MainActor @objc func submitFromAction(_ sender: NSTextField) {
            submit(isShiftPressed: NSApp.currentEvent?.modifierFlags.contains(.shift) == true)
        }
    }
}

final class SearchNSTextField: NSTextField {
    var onSubmit: ((Bool) -> Void)?
    var onClose: (() -> Void)?

    override func keyDown(with event: NSEvent) {
        switch event.keyCode {
        case 36, 76:
            onSubmit?(event.modifierFlags.contains(.shift))
        case 53:
            onClose?()
        default:
            super.keyDown(with: event)
        }
    }
}
