#!/bin/sh
# A tiny Agent Client Protocol agent for tests and screenshots. It speaks newline-delimited
# JSON-RPC on stdio: initialize, session/new (with a model selector), session/set_config_option
# and session/prompt, streaming a thought, a tool call and a markdown answer that cites the
# first source Mdow sent.
#
# Prompts containing PERMISSION ask for a tool permission first; SLOW streams one chunk and never
# finishes (for Stop). FAKE_ACP_MODE=auth rejects session/new as unauthenticated.

model="opencode/claude-sonnet-4-5"
pending_prompt=""
source_id=""

models() {
  printf '[{"id":"model","name":"Model","category":"model","type":"select","currentValue":"%s","options":[{"value":"openai/gpt-5.4","name":"GPT-5.4"},{"value":"opencode/claude-sonnet-4-5","name":"Claude Sonnet 4.5"},{"value":"opencode-go/kimi-k2.5","name":"Kimi K2.5"}]}]' "$model"
}

update() {
  printf '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fake-session","update":%s}}\n' "$1"
}

chunk() {
  update "{\"sessionUpdate\":\"agent_message_chunk\",\"content\":{\"type\":\"text\",\"text\":\"$1\"}}"
}

answer() {
  update '{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"Reading the focused document and its headings."}}'
  update '{"sessionUpdate":"tool_call","toolCallId":"read-1","title":"Read showcase.md","kind":"read","status":"pending","rawInput":{"path":"showcase.md"}}'
  update '{"sessionUpdate":"tool_call_update","toolCallId":"read-1","status":"completed","rawOutput":{"lines":42}}'
  chunk "## Summary\\n\\nThis document is a **tour of Mdow's markdown rendering**"
  if [ -n "$source_id" ]; then
    chunk " ($source_id)"
  fi
  chunk ".\\n\\n- Headings, lists and \`inline code\`\\n- Fenced code with syntax highlighting\\n- Tables and callouts\\n\\n"
  chunk "\`\`\`rust\\nfn main() {\\n    println!(\\\"hello from the companion\\\");\\n}\\n\`\`\`\\n\\n"
  chunk "> Ask a follow-up to go deeper into any section."
}

finish() {
  printf '{"jsonrpc":"2.0","id":%s,"result":{"stopReason":"end_turn"}}\n' "$1"
}

while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/^{\("jsonrpc":"2.0",\)\{0,1\}"id":\([0-9][0-9]*\),.*/\2/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":1,"agentCapabilities":{"loadSession":false},"authMethods":[]}}\n' "$id"
      ;;
    *'"method":"session/new"'*)
      if [ "$FAKE_ACP_MODE" = "auth" ]; then
        printf '{"jsonrpc":"2.0","id":%s,"error":{"code":-32000,"message":"Authentication required"}}\n' "$id"
      else
        printf '{"jsonrpc":"2.0","id":%s,"result":{"sessionId":"fake-session","configOptions":%s}}\n' "$id" "$(models)"
      fi
      ;;
    *'"method":"session/set_config_option"'*)
      value=$(printf '%s' "$line" | sed -n 's/.*"value":"\([^"]*\)".*/\1/p')
      [ -n "$value" ] && model="$value"
      printf '{"jsonrpc":"2.0","id":%s,"result":{"configOptions":%s}}\n' "$id" "$(models)"
      ;;
    *'"method":"session/cancel"'*)
      ;;
    *'"method":"session/prompt"'*)
      source_id=$(printf '%s' "$line" | sed -n 's/.*### Source \(src:[^\\]*\)\\nPath.*/\1/p')
      case "$line" in
        *SLOW*)
          chunk "Working on it"
          ;;
        *PERMISSION*)
          pending_prompt="$id"
          printf '{"jsonrpc":"2.0","id":900,"method":"session/request_permission","params":{"sessionId":"fake-session","toolCall":{"toolCallId":"edit-1","title":"Edit notes.md","rawInput":{"path":"notes.md","newText":"Updated"}},"options":[{"optionId":"allow","name":"Allow","kind":"allow_once"},{"optionId":"reject","name":"Reject","kind":"reject_once"}]}}\n'
          ;;
        *)
          answer
          finish "$id"
          ;;
      esac
      ;;
    *'"result":{"outcome"'*)
      if [ -n "$pending_prompt" ]; then
        case "$line" in
          *'"optionId":"allow"'*) chunk "Permission granted, but Mdow keeps the companion read-only." ;;
          *) chunk "Understood. I left the file unchanged." ;;
        esac
        finish "$pending_prompt"
        pending_prompt=""
      fi
      ;;
  esac
done
