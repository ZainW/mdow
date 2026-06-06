import Foundation

public struct MermaidDiagram: Equatable, Sendable {
    public let direction: MermaidDirection
    public let nodes: [MermaidNode]
    public let edges: [MermaidEdge]
    public let source: String

    public init(
        direction: MermaidDirection,
        nodes: [MermaidNode],
        edges: [MermaidEdge],
        source: String
    ) {
        self.direction = direction
        self.nodes = nodes
        self.edges = edges
        self.source = source
    }
}

public enum MermaidDirection: Equatable, Sendable {
    case topDown
    case leftToRight
    case rightToLeft
    case bottomTop
}

public struct MermaidNode: Equatable, Identifiable, Sendable {
    public let id: String
    public let label: String

    public init(id: String, label: String) {
        self.id = id
        self.label = label
    }
}

public struct MermaidEdge: Equatable, Sendable {
    public let from: String
    public let to: String

    public init(from: String, to: String) {
        self.from = from
        self.to = to
    }
}

public enum MermaidDiagramParser {
    public static func parse(_ source: String) -> MermaidDiagram? {
        let lines = source.components(separatedBy: .newlines)
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty && !$0.hasPrefix("%%") }

        guard let firstLine = lines.first else { return nil }
        let headerParts = firstLine.split(separator: " ", maxSplits: 1).map(String.init)
        guard headerParts.count == 2,
              headerParts[0] == "flowchart" || headerParts[0] == "graph",
              let direction = parseDirection(headerParts[1]) else {
            return nil
        }

        var nodesByID: [String: MermaidNode] = [:]
        var nodeOrder: [String] = []
        var edges: [MermaidEdge] = []

        func addNode(_ node: MermaidNode) {
            guard nodesByID[node.id] == nil else { return }
            nodesByID[node.id] = node
            nodeOrder.append(node.id)
        }

        for line in lines.dropFirst() {
            guard let arrowRange = line.range(of: "-->") else { continue }
            let leftText = String(line[..<arrowRange.lowerBound]).trimmingCharacters(in: .whitespaces)
            let rightText = String(line[arrowRange.upperBound...]).trimmingCharacters(in: .whitespaces)
            guard let leftNode = parseNode(leftText),
                  let rightNode = parseNode(rightText) else {
                continue
            }

            addNode(leftNode)
            addNode(rightNode)
            edges.append(MermaidEdge(from: leftNode.id, to: rightNode.id))
        }

        guard !nodesByID.isEmpty, !edges.isEmpty else { return nil }
        let nodes = nodeOrder.compactMap { nodesByID[$0] }
        return MermaidDiagram(direction: direction, nodes: nodes, edges: edges, source: source)
    }

    private static func parseDirection(_ text: String) -> MermaidDirection? {
        switch text.uppercased() {
        case "TD", "TB":
            .topDown
        case "LR":
            .leftToRight
        case "RL":
            .rightToLeft
        case "BT":
            .bottomTop
        default:
            nil
        }
    }

    private static func parseNode(_ text: String) -> MermaidNode? {
        let trimmed = text.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return nil }

        if let openBracket = trimmed.firstIndex(of: "["),
           let closeBracket = trimmed.lastIndex(of: "]"),
           openBracket < closeBracket {
            let id = String(trimmed[..<openBracket]).trimmingCharacters(in: .whitespaces)
            let labelStart = trimmed.index(after: openBracket)
            let label = String(trimmed[labelStart..<closeBracket])
            guard !id.isEmpty, !label.isEmpty else { return nil }
            return MermaidNode(id: id, label: label)
        }

        return MermaidNode(id: trimmed, label: trimmed)
    }
}
