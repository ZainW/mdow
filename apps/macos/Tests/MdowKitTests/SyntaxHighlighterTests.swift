import Foundation
import Testing

@testable import MdowKit

/// The kind of the token that fully covers the `occurrence`th match of `needle`, or `.plain`.
private func kind(
  of needle: String, in code: String, _ language: String, occurrence: Int = 1
) -> TokenKind {
  let ns = code as NSString
  var search = NSRange(location: 0, length: ns.length)
  var found = NSRange(location: NSNotFound, length: 0)
  for _ in 0..<occurrence {
    found = ns.range(of: needle, options: [], range: search)
    guard found.location != NSNotFound else { return .plain }
    search = NSRange(location: NSMaxRange(found), length: ns.length - NSMaxRange(found))
  }
  let tokens = SyntaxHighlighter.tokens(in: code, language: language)
  for token in tokens
  where token.range.location <= found.location && NSMaxRange(token.range) >= NSMaxRange(found) {
    return token.kind
  }
  return .plain
}

private func checkInvariants(_ code: String, _ language: String) {
  let tokens = SyntaxHighlighter.tokens(in: code, language: language)
  let length = (code as NSString).length
  var end = 0
  for token in tokens {
    #expect(token.range.location >= end, "overlap in \(language)")
    #expect(token.range.length > 0, "empty token in \(language)")
    #expect(NSMaxRange(token.range) <= length, "out of bounds in \(language)")
    #expect(token.kind != .plain)
    end = NSMaxRange(token.range)
  }
}

@Suite struct SyntaxHighlighterTests {
  @Test func languageAliases() {
    let cases: [(String?, String?)] = [
      ("ts", "typescript"), ("TypeScript", "typescript"), ("js {1,3}", "javascript"),
      ("c++", "cpp"), ("sh", "bash"), ("zsh", "bash"), ("yml", "yaml"), ("objc", "objectivec"),
      ("jsonc", "jsonc"), ("json5", "jsonc"), ("py", "python"), ("rs", "rust"),
      ("golang", "go"), ("C#", "csharp"), ("kt", "kotlin"), ("rb", "ruby"),
      ("ps1", "powershell"), ("postgres", "sql"), ("svg", "svg"), ("scss", "scss"),
      ("md", "markdown"), ("patch", "diff"), ("Dockerfile", "dockerfile"),
      ("makefile", "makefile"), ("ex", "elixir"), ("hs", "haskell"), ("gql", "graphql"),
      ("r", "r"), ("pl", "perl"), ("language-python", "python"), ("{.python}", "python"),
      ("python title=\"app.py\"", "python"), ("main.rs", "rust"), ("console", "console"),
      ("shell-session", "console"), ("tsx", "tsx"), ("jsx", "jsx"), ("text", "plaintext"),
      ("  swift  ", "swift"), ("js:app.js", "javascript"), (nil, nil), ("", nil),
      ("klingon", nil),
    ]
    for (info, expected) in cases {
      #expect(SyntaxHighlighter.language(for: info) == expected, "\(info ?? "nil")")
    }
  }

  @Test func displayNames() {
    #expect(SyntaxHighlighter.displayName(for: "typescript") == "TypeScript")
    #expect(SyntaxHighlighter.displayName(for: "cpp") == "C++")
    #expect(SyntaxHighlighter.displayName(for: "csharp") == "C#")
    #expect(SyntaxHighlighter.displayName(for: "objectivec") == "Objective-C")
    for id in SyntaxHighlighter.allLanguages {
      #expect(!SyntaxHighlighter.displayName(for: id).isEmpty)
      #expect(SyntaxHighlighter.language(for: id) == id, "\(id) is not its own alias")
    }
  }

  @Test func unknownLanguageHasNoTokens() {
    #expect(SyntaxHighlighter.tokens(in: "let x = 1", language: "klingon").isEmpty)
    #expect(SyntaxHighlighter.tokens(in: "let x = 1", language: nil).isEmpty)
    #expect(SyntaxHighlighter.tokens(in: "let x = 1", language: "plaintext").isEmpty)
    #expect(SyntaxHighlighter.tokens(in: "", language: "swift").isEmpty)
  }

  @Test func acceptsAliasesDirectly() {
    #expect(kind(of: "const", in: "const a = 1", "ts") == .keyword)
  }

  @Test func swift() {
    let code = """
      import SwiftUI
      @MainActor
      struct Greeter: View {
        // A comment
        let count = 0x1F_FF
        func greet(name: String) -> String {
          return "Hello, \\(name.uppercased())!" /* done */
        }
      }
      #if DEBUG
      let raw = #"a "quoted" \\n"#
      #endif
      """
    #expect(kind(of: "import", in: code, "swift") == .keyword)
    #expect(kind(of: "@MainActor", in: code, "swift") == .attribute)
    #expect(kind(of: "Greeter", in: code, "swift") == .type)
    #expect(kind(of: "View", in: code, "swift") == .type)
    #expect(kind(of: "// A comment", in: code, "swift") == .comment)
    #expect(kind(of: "0x1F_FF", in: code, "swift") == .number)
    #expect(kind(of: "greet", in: code, "swift") == .function)
    #expect(kind(of: "\"Hello, ", in: code, "swift") == .string)
    #expect(kind(of: "name", in: code, "swift", occurrence: 2) == .plain)
    #expect(kind(of: "uppercased", in: code, "swift") == .function)
    #expect(kind(of: "!\"", in: code, "swift") == .string)
    #expect(kind(of: "/* done */", in: code, "swift") == .comment)
    #expect(kind(of: "#if", in: code, "swift") == .preprocessor)
    #expect(kind(of: "#\"a \"quoted\" \\n\"#", in: code, "swift") == .string)
  }

  @Test func typescriptAndJSX() {
    let ts = """
      interface User { name: string; age?: number }
      const re = /ab+c/gi, half = total / 2
      export async function load(id: number): Promise<User> {
        const url = `https://api/${id}/user`
        return fetch(url).then((r) => r.json()) // fetch
      }
      """
    #expect(kind(of: "interface", in: ts, "typescript") == .keyword)
    #expect(kind(of: "User", in: ts, "typescript") == .type)
    #expect(kind(of: "string", in: ts, "typescript") == .type)
    #expect(kind(of: "/ab+c/gi", in: ts, "typescript") == .regex)
    #expect(kind(of: "/ 2", in: ts, "typescript") == .plain)
    #expect(kind(of: "load", in: ts, "typescript") == .function)
    #expect(kind(of: "`https://api/", in: ts, "typescript") == .string)
    #expect(kind(of: "id", in: ts, "typescript", occurrence: 3) == .plain)
    #expect(kind(of: "/user`", in: ts, "typescript") == .string)
    #expect(kind(of: "json", in: ts, "typescript") == .function)
    #expect(kind(of: "// fetch", in: ts, "typescript") == .comment)

    let tsx = """
      export function App({ items }) {
        return (
          <ul className="list" onClick={() => go(1)}>
            {items.map((item) => <li key={item.id}>{item.name}</li>)}
          </ul>
        )
      }
      """
    #expect(kind(of: "<ul", in: tsx, "tsx") == .tag)
    #expect(kind(of: "className", in: tsx, "tsx") == .attribute)
    #expect(kind(of: "\"list\"", in: tsx, "tsx") == .string)
    #expect(kind(of: "go", in: tsx, "tsx") == .function)
    #expect(kind(of: "1", in: tsx, "tsx") == .number)
    #expect(kind(of: "<li", in: tsx, "tsx") == .tag)
    #expect(kind(of: "</li>", in: tsx, "tsx") == .tag)
    #expect(kind(of: "</ul>", in: tsx, "tsx") == .tag)
    #expect(kind(of: "map", in: tsx, "tsx") == .function)
    #expect(kind(of: "return", in: tsx, "tsx") == .keyword)
  }

  @Test func python() {
    let code = """
      @dataclass
      class Point(Base):
          \"\"\"A point.\"\"\"
          def norm(self) -> float:  # length
              return (self.x ** 2 + 1.5e3) if self else None
      print(f"x={p.x}", r'\\d+')
      """
    #expect(kind(of: "@dataclass", in: code, "python") == .attribute)
    #expect(kind(of: "class", in: code, "python", occurrence: 2) == .keyword)
    #expect(kind(of: "Point", in: code, "python") == .type)
    #expect(kind(of: "\"\"\"A point.\"\"\"", in: code, "python") == .string)
    #expect(kind(of: "norm", in: code, "python") == .function)
    #expect(kind(of: "self", in: code, "python") == .constant)
    #expect(kind(of: "float", in: code, "python") == .type)
    #expect(kind(of: "# length", in: code, "python") == .comment)
    #expect(kind(of: "x", in: code, "python", occurrence: 1) == .property)
    #expect(kind(of: "1.5e3", in: code, "python") == .number)
    #expect(kind(of: "None", in: code, "python") == .constant)
    #expect(kind(of: "print", in: code, "python") == .function)
    #expect(kind(of: "f\"x={p.x}\"", in: code, "python") == .string)
    #expect(kind(of: "r'\\d+'", in: code, "python") == .string)
  }

  @Test func rust() {
    let code = """
      #[derive(Debug, Clone)]
      pub struct Wrapper<'a> { inner: &'a str, c: char }
      fn main() {
          let c = 'x';
          let s = r#"raw "string""#;
          println!("{} {}", c, 42u8);
      }
      """
    #expect(kind(of: "#[derive(Debug, Clone)]", in: code, "rust") == .attribute)
    #expect(kind(of: "struct", in: code, "rust") == .keyword)
    #expect(kind(of: "Wrapper", in: code, "rust") == .type)
    #expect(kind(of: "'a", in: code, "rust") == .attribute)
    #expect(kind(of: "str", in: code, "rust", occurrence: 2) == .type)
    #expect(kind(of: "main", in: code, "rust") == .function)
    #expect(kind(of: "'x'", in: code, "rust") == .string)
    #expect(kind(of: "r#\"raw \"string\"\"#", in: code, "rust") == .string)
    #expect(kind(of: "println!", in: code, "rust") == .function)
    #expect(kind(of: "42u8", in: code, "rust") == .number)
  }

  @Test func cFamily() {
    let c = """
      #include <stdio.h>
      #define MAX 10
      int main(void) {
        char *s = "hi\\n"; // greet
        printf("%s %d\\n", s, 'c');
        return 0;
      }
      """
    #expect(kind(of: "#include", in: c, "c") == .preprocessor)
    #expect(kind(of: "<stdio.h>", in: c, "c") == .string)
    #expect(kind(of: "#define", in: c, "c") == .preprocessor)
    #expect(kind(of: "int", in: c, "c") == .type)
    #expect(kind(of: "main", in: c, "c") == .function)
    #expect(kind(of: "\"hi\\n\"", in: c, "c") == .string)
    #expect(kind(of: "'c'", in: c, "c") == .string)
    #expect(kind(of: "return", in: c, "c") == .keyword)

    let objc = "@interface Foo : NSObject\n@property NSString *name;\n@end\nNSLog(@\"hi %@\", YES);"
    #expect(kind(of: "@interface", in: objc, "objc") == .keyword)
    #expect(kind(of: "@\"hi %@\"", in: objc, "objc") == .string)
    #expect(kind(of: "YES", in: objc, "objc") == .constant)
  }

  @Test func json() {
    let code = """
      {
        // comment
        "name": "mdow",
        "version": 1.5,
        "private": true,
        "tags": ["a", null]
      }
      """
    #expect(kind(of: "\"name\"", in: code, "jsonc") == .property)
    #expect(kind(of: "\"mdow\"", in: code, "jsonc") == .string)
    #expect(kind(of: "1.5", in: code, "jsonc") == .number)
    #expect(kind(of: "true", in: code, "jsonc") == .constant)
    #expect(kind(of: "null", in: code, "jsonc") == .constant)
    #expect(kind(of: "// comment", in: code, "jsonc") == .comment)
  }

  @Test func shell() {
    let code = """
      #!/usr/bin/env bash
      $ npm install --save mdow # install
      export PATH="$HOME/bin:$PATH"
      if [ -f "$FILE" ]; then
        echo "found ${FILE}" | grep -q found && sudo rm -rf /tmp/x
      fi
      cat <<EOF > out.txt
      hello $USER
      EOF
      ls
      """
    #expect(kind(of: "#!/usr/bin/env bash", in: code, "bash") == .comment)
    #expect(kind(of: "$", in: code, "bash", occurrence: 1) == .comment)
    #expect(kind(of: "npm", in: code, "bash") == .function)
    #expect(kind(of: "install", in: code, "bash") == .plain)
    #expect(kind(of: "# install", in: code, "bash") == .comment)
    #expect(kind(of: "export", in: code, "bash") == .keyword)
    #expect(kind(of: "PATH", in: code, "bash") == .property)
    #expect(kind(of: "$HOME", in: code, "bash") == .property)
    #expect(kind(of: "/bin:", in: code, "bash") == .string)
    #expect(kind(of: "if", in: code, "bash") == .keyword)
    #expect(kind(of: "echo", in: code, "bash") == .function)
    #expect(kind(of: "${FILE}", in: code, "bash") == .property)
    #expect(kind(of: "grep", in: code, "bash") == .function)
    #expect(kind(of: "rm", in: code, "bash") == .function)
    #expect(kind(of: "hello", in: code, "bash") == .string)
    #expect(kind(of: "ls", in: code, "bash") == .function)

    let console = "$ brew install mdow\n==> Downloading\nuser@host:~$ ls -la\ntotal 0"
    #expect(kind(of: "brew", in: console, "console") == .function)
    #expect(kind(of: "Downloading", in: console, "console") == .plain)
    #expect(kind(of: "user@host:~$", in: console, "console") == .comment)
    #expect(kind(of: "ls", in: console, "console") == .function)
  }

  @Test func markupAndCSS() {
    let html = """
      <!DOCTYPE html>
      <!-- note -->
      <div class="box" hidden data-x='1'>Tom &amp; Jerry</div>
      <style>.box { color: #fff; margin: 0 auto; }</style>
      <script>const x = 1</script>
      """
    #expect(kind(of: "<!DOCTYPE html>", in: html, "html") == .preprocessor)
    #expect(kind(of: "<!-- note -->", in: html, "html") == .comment)
    #expect(kind(of: "<div", in: html, "html") == .tag)
    #expect(kind(of: "class", in: html, "html") == .attribute)
    #expect(kind(of: "\"box\"", in: html, "html") == .string)
    #expect(kind(of: "hidden", in: html, "html") == .attribute)
    #expect(kind(of: "&amp;", in: html, "html") == .constant)
    #expect(kind(of: "Jerry", in: html, "html") == .plain)
    #expect(kind(of: "</div>", in: html, "html") == .tag)
    #expect(kind(of: ".box", in: html, "html", occurrence: 1) == .type)
    #expect(kind(of: "color", in: html, "html") == .property)
    #expect(kind(of: "#fff", in: html, "html") == .number)
    #expect(kind(of: "const", in: html, "html") == .keyword)

    let css = """
      @media (max-width: 600px) {
        nav > a:hover, .btn::before { padding: 1.5em -2px; background: url(img.png) !important; }
      }
      """
    #expect(kind(of: "@media", in: css, "css") == .keyword)
    #expect(kind(of: "max-width", in: css, "css") == .property)
    #expect(kind(of: "600px", in: css, "css") == .number)
    #expect(kind(of: "nav", in: css, "css") == .tag)
    #expect(kind(of: ":hover", in: css, "css") == .attribute)
    #expect(kind(of: ".btn", in: css, "css") == .type)
    #expect(kind(of: "padding", in: css, "css") == .property)
    #expect(kind(of: "-2px", in: css, "css") == .number)
    #expect(kind(of: "url", in: css, "css") == .function)
    #expect(kind(of: "img.png", in: css, "css") == .string)
    #expect(kind(of: "!important", in: css, "css") == .keyword)
  }

  @Test func dataFormats() {
    let yaml = """
      # config
      name: mdow
      version: 1.2
      enabled: true
      list:
        - "quoted"
        - plain value # trailing
      anchors: &base { a: 1, b: [x, 2] }
      script: |
        echo hi
        echo there
      after: ok
      """
    #expect(kind(of: "# config", in: yaml, "yaml") == .comment)
    #expect(kind(of: "name", in: yaml, "yaml") == .property)
    #expect(kind(of: "mdow", in: yaml, "yaml") == .string)
    #expect(kind(of: "1.2", in: yaml, "yaml") == .number)
    #expect(kind(of: "true", in: yaml, "yaml") == .constant)
    #expect(kind(of: "\"quoted\"", in: yaml, "yaml") == .string)
    #expect(kind(of: "plain value", in: yaml, "yaml") == .string)
    #expect(kind(of: "# trailing", in: yaml, "yaml") == .comment)
    #expect(kind(of: "&base", in: yaml, "yaml") == .attribute)
    #expect(kind(of: "a", in: "x: { a: 1 }", "yaml") == .property)
    #expect(kind(of: "echo there", in: yaml, "yaml") == .string)
    #expect(kind(of: "after", in: yaml, "yaml") == .property)

    let toml = "[package]\nname = \"mdow\" # n\nversion = 3\n[[bin]]\nfeatures = [\"a\"]"
    #expect(kind(of: "[package]", in: toml, "toml") == .type)
    #expect(kind(of: "name", in: toml, "toml") == .property)
    #expect(kind(of: "\"mdow\"", in: toml, "toml") == .string)
    #expect(kind(of: "# n", in: toml, "toml") == .comment)
    #expect(kind(of: "3", in: toml, "toml") == .number)
    #expect(kind(of: "[[bin]]", in: toml, "toml") == .type)

    let ini = "; c\n[core]\neditor = vim\nenabled = true"
    #expect(kind(of: "[core]", in: ini, "ini") == .type)
    #expect(kind(of: "editor", in: ini, "ini") == .property)
    #expect(kind(of: "true", in: ini, "ini") == .constant)

    let sql = "SELECT id, COUNT(*) FROM users WHERE name = 'O''Brien' -- c\nLIMIT 10;"
    #expect(kind(of: "SELECT", in: sql, "sql") == .keyword)
    #expect(kind(of: "COUNT", in: sql, "sql") == .function)
    #expect(kind(of: "'O''Brien'", in: sql, "sql") == .string)
    #expect(kind(of: "-- c", in: sql, "sql") == .comment)
    #expect(kind(of: "10", in: sql, "sql") == .number)
  }

  @Test func markdownAndDiff() {
    let md = """
      # Title
      Some **bold** and `code` with [a link](https://x.y).
      - item
      ```swift
      let x = 1
      ```
      """
    #expect(kind(of: "# Title", in: md, "markdown") == .heading)
    #expect(kind(of: "**bold**", in: md, "markdown") == .keyword)
    #expect(kind(of: "`code`", in: md, "markdown") == .string)
    #expect(kind(of: "[a link](https://x.y)", in: md, "markdown") == .link)
    #expect(kind(of: "-", in: md, "markdown") == .keyword)
    #expect(kind(of: "let", in: md, "markdown") == .keyword)
    #expect(kind(of: "1", in: md, "markdown") == .number)

    let diff = """
      diff --git a/f b/f
      --- a/f
      +++ b/f
      @@ -1,2 +1,2 @@ func
       context
      -old
      +new
      """
    #expect(kind(of: "--- a/f", in: diff, "diff") == .heading)
    #expect(kind(of: "@@ -1,2 +1,2 @@", in: diff, "diff") == .preprocessor)
    #expect(kind(of: "context", in: diff, "diff") == .plain)
    #expect(kind(of: "-old", in: diff, "diff") == .deleted)
    #expect(kind(of: "+new", in: diff, "diff") == .inserted)
  }

  @Test func otherLanguages() {
    let go = "package main\nfunc main() {\n\ts := `raw`\n\tfmt.Println(s, nil)\n}"
    #expect(kind(of: "package", in: go, "go") == .keyword)
    #expect(kind(of: "`raw`", in: go, "go") == .string)
    #expect(kind(of: "Println", in: go, "go") == .function)
    #expect(kind(of: "nil", in: go, "go") == .constant)

    let ruby = "class Foo < Bar\n  def ok?(x) = @name || :sym || {key: 1}\nend # done"
    #expect(kind(of: "Foo", in: ruby, "ruby") == .type)
    #expect(kind(of: "ok?", in: ruby, "ruby") == .function)
    #expect(kind(of: "@name", in: ruby, "ruby") == .property)
    #expect(kind(of: ":sym", in: ruby, "ruby") == .constant)
    #expect(kind(of: "key:", in: ruby, "ruby") == .constant)
    #expect(kind(of: "# done", in: ruby, "ruby") == .comment)

    let php = "<p><?php echo $name; ?></p>"
    #expect(kind(of: "<p>", in: php, "php") == .tag)
    #expect(kind(of: "<?php", in: php, "php") == .preprocessor)
    #expect(kind(of: "echo", in: php, "php") == .keyword)
    #expect(kind(of: "$name", in: php, "php") == .property)

    let ps = "Get-ChildItem -Path $env:HOME | Where-Object { $_.Length -gt 1 } # c"
    #expect(kind(of: "Get-ChildItem", in: ps, "powershell") == .function)
    #expect(kind(of: "-Path", in: ps, "powershell") == .attribute)
    #expect(kind(of: "# c", in: ps, "powershell") == .comment)

    let docker =
      "FROM node:20 AS build\nRUN npm ci && \\\n    npm run build\nCMD [\"node\", \"x.js\"]"
    #expect(kind(of: "FROM", in: docker, "dockerfile") == .keyword)
    #expect(kind(of: "AS", in: docker, "dockerfile") == .keyword)
    #expect(kind(of: "npm", in: docker, "dockerfile") == .function)
    #expect(kind(of: "npm", in: docker, "dockerfile", occurrence: 2) == .function)
    #expect(kind(of: "\"node\"", in: docker, "dockerfile") == .string)

    let make = "CC := clang\n.PHONY: all\nall: $(OBJ)\n\t@$(CC) -o app main.c # build\n"
    #expect(kind(of: "CC", in: make, "makefile") == .property)
    #expect(kind(of: ".PHONY", in: make, "makefile") == .keyword)
    #expect(kind(of: "all", in: make, "makefile", occurrence: 2) == .function)
    #expect(kind(of: "$(OBJ)", in: make, "makefile") == .property)
    #expect(kind(of: "$(CC)", in: make, "makefile") == .property)

    let lua = "--[[ block ]]\nlocal function f(t) return #t end -- c"
    #expect(kind(of: "--[[ block ]]", in: lua, "lua") == .comment)
    #expect(kind(of: "local", in: lua, "lua") == .keyword)
    #expect(kind(of: "f", in: lua, "lua", occurrence: 2) == .function)

    let elixir = "defmodule Foo do\n  @doc \"hi #{name}\"\n  def bar(x), do: ~r/ab+/i\nend"
    #expect(kind(of: "Foo", in: elixir, "elixir") == .type)
    #expect(kind(of: "@doc", in: elixir, "elixir") == .attribute)
    #expect(kind(of: "bar", in: elixir, "elixir") == .function)
    #expect(kind(of: "do:", in: elixir, "elixir") == .constant)
    #expect(kind(of: "~r/ab+/i", in: elixir, "elixir") == .regex)

    let haskell = "main :: IO ()\nmain = putStrLn \"hi\" -- greet\n{- block {- nested -} -}"
    #expect(kind(of: "main", in: haskell, "haskell") == .function)
    #expect(kind(of: "IO", in: haskell, "haskell") == .type)
    #expect(kind(of: "{- block {- nested -} -}", in: haskell, "haskell") == .comment)

    let kotlin = "fun greet(name: String) = println(\"Hi $name, ${name.length}\")"
    #expect(kind(of: "fun", in: kotlin, "kotlin") == .keyword)
    #expect(kind(of: "$name", in: kotlin, "kotlin") == .property)
    #expect(kind(of: "length", in: kotlin, "kotlin") == .property)

    let perl = "my $x = shift; if ($x =~ m/^a+$/i) { print \"yes\" }"
    #expect(kind(of: "$x", in: perl, "perl") == .property)
    #expect(kind(of: "m/^a+$/i", in: perl, "perl") == .regex)

    let zig = "const std = @import(\"std\");\nconst s = \\\\multi\n;"
    #expect(kind(of: "@import", in: zig, "zig") == .function)
    #expect(kind(of: "\\\\multi", in: zig, "zig") == .string)

    let nix = "{ pkgs ? import <nixpkgs> {} }: let x = \"${pkgs.hello}\"; in x"
    #expect(kind(of: "let", in: nix, "nix") == .keyword)
    #expect(kind(of: "x", in: nix, "nix", occurrence: 2) == .property)

    let gql = "query GetUser($id: ID!) { user(id: $id) @include(if: true) { name } }"
    #expect(kind(of: "query", in: gql, "graphql") == .keyword)
    #expect(kind(of: "GetUser", in: gql, "graphql") == .function)
    #expect(kind(of: "$id", in: gql, "graphql") == .property)
    #expect(kind(of: "@include", in: gql, "graphql") == .attribute)

    let csharp = "var s = $\"Hi {name}\"; var v = @\"C:\\path\";\n#region X"
    #expect(kind(of: "$\"Hi ", in: csharp, "csharp") == .string)
    #expect(kind(of: "@\"C:\\path\"", in: csharp, "csharp") == .string)
    #expect(kind(of: "#region", in: csharp, "csharp") == .preprocessor)
  }

  @Test func unterminatedInputDoesNotCrash() {
    let nasty = [
      "\"abc", "'abc", "`abc ${", "`abc ${ x", "/* abc", "{- abc", "<!-- abc", "<div attr=\"",
      "<div", "<", "${", "$(", "\"\\", "\\", "#", "#[", "r#\"abc", "'''abc", "\"\"\"abc", "@",
      "0x", "1e", ".", "<<EOF\nbody", "```swift\nlet x", "---\na: b", "a: |\n  x", "[", "[[",
      "=begin\nabc", "<?php echo", "<script>let x", "<style>a {", "\\(", "f(\"\\(\"", "~r/",
      "m{", "s/a/", "@\"", "$\"{", "#\"", "##\"\"\"", "R\"x(", "'\\", "'", "<a>{", "<></",
      "{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{", "\u{0}", "\r\n\r\n", "\t",
    ]
    for language in SyntaxHighlighter.allLanguages {
      for code in nasty {
        checkInvariants(code, language)
      }
    }
  }

  @Test func deepNestingDoesNotOverflow() {
    let deep = String(repeating: "`${", count: 5000)
    checkInvariants(deep, "typescript")
    checkInvariants(String(repeating: "\"\\(", count: 5000), "swift")
    checkInvariants(String(repeating: "<a>", count: 5000), "tsx")
    checkInvariants(String(repeating: "$(", count: 5000), "bash")
    checkInvariants(String(repeating: "```md\n", count: 2000), "markdown")
  }

  @Test func mixedSampleKeepsInvariantsEverywhere() {
    let sample = """
      // comment # comment -- comment ; comment
      let x = "str" + 'c' + `tpl ${y}` + 0x1F + 1_000.5e-3 // 😀 emoji
      #include <a.h>
      @attr func f(a: Int) -> String { return a.b(c) }
      <div class="x">text</div> $var ${var} :sym key: value
      /* block 🎉 */ [section] name = value
      - item: [1, 2, {a: b}]
      """
    for language in SyntaxHighlighter.allLanguages {
      checkInvariants(sample, language)
    }
  }

  @Test func nonBMPTextGivesUTF16Ranges() {
    let code = "let s = \"😀👍🏽\" // 🎉 done\nlet n = 1"
    let tokens = SyntaxHighlighter.tokens(in: code, language: "swift")
    let ns = code as NSString
    let string = tokens.first { $0.kind == .string }
    #expect(string?.range == ns.range(of: "\"😀👍🏽\""))
    let comment = tokens.first { $0.kind == .comment }
    #expect(comment?.range == ns.range(of: "// 🎉 done"))
    let number = tokens.first { $0.kind == .number }
    #expect(number?.range == ns.range(of: "1"))
    #expect(kind(of: "let", in: code, "swift", occurrence: 2) == .keyword)

    let md = "# Héllo 😀\n`😀` text"
    #expect(kind(of: "# Héllo 😀", in: md, "markdown") == .heading)
    #expect(kind(of: "`😀`", in: md, "markdown") == .string)

    let rust = "let c = '😀'; let d = 'é';"
    #expect(kind(of: "'😀'", in: rust, "rust") == .string)
    #expect(kind(of: "'é'", in: rust, "rust") == .string)
  }

  @Test func performance() {
    let chunk = """
      import { readFile } from 'node:fs/promises'
      // Load and parse the configuration file.
      export async function loadConfig(path: string): Promise<Config> {
        const text = await readFile(path, 'utf8')
        const value = JSON.parse(text) as Partial<Config>
        if (!value.name || value.retries > 10) throw new Error(`bad config: ${path}`)
        return { ...defaults, ...value, pattern: /^[a-z]+$/i, ratio: 0.75 * 2 }
      }

      """
    let lines = chunk.split(separator: "\n", omittingEmptySubsequences: false).count - 1
    let code = String(repeating: chunk, count: 20_000 / lines)
    let clock = ContinuousClock()
    var count = 0
    let elapsed = clock.measure {
      count = SyntaxHighlighter.tokens(in: code, language: "typescript").count
    }
    #expect(count > 100_000)
    #expect(elapsed < .seconds(3), "20k lines took \(elapsed)")
  }
}
