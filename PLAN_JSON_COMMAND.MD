# Plan: AI JSON Response with Commands Support

## Overview
Extend the AI response system to parse JSON responses, extracting both text and commands. If the AI responds with valid JSON, extract and execute commands (like `doEmote`). Otherwise, proceed with current plain text behavior.

## Changes Required

### 1. Add Command Context to AI Prompt (`AiHandler.hx`)

**New function `getCommandContext():String`**
- Returns information about available commands the AI can use
- Include 15 valid emotes: `happy`, `angry`, `love`, `sad`, `joy`, `blush`, `devious`, `shock`, `terrified`, `homesick`, `mad`, `oreally`, `ill`, `hmph`, `snowSplat`

**Modified `buildPrompt()`**
- Add command context to the prompt so AI knows available commands

### 2. JSON Response Parsing and Command Execution (`AiHandler.hx`)

**New function `parseAiResponse(response:String, aiPlayer:GlobalPlayerInstance):String`**
- Try to parse response as JSON using `haxe.Json.parse()`
- If valid JSON, extract:
  - `text` field: the response text to speak
  - `command` field: if present, contains `{type: "doEmote", emote: "emoteName"}`
- If not valid JSON, return original response
- If command is `doEmote`, call `aiPlayer.doEmote(Emote.emoteName)`

**JSON Format expected from AI:**
```json
{
  "text": "Response text here",
  "command": {
    "type": "doEmote",
    "emote": "happy"
  }
}
```

Or plain text (current behavior):
```
Just a normal response text
```

### 3. Modify `respondToPlayerAsync` to Parse Before `sendResponseInChunks`

In `AiHandler.respondToPlayerAsync()`:
- Call `parseAiResponse(response, fromPlayer)` BEFORE `sendResponseInChunks()`
- Pass the parsed text (extracted from JSON if applicable) to `sendResponseInChunks()`
- This ensures commands are executed and text is extracted before chunking/sending

**Flow:**
1. `ChatResponse()` returns raw response
2. `parseAiResponse(response, fromPlayer)` - extracts text, executes commands
3. `sendResponseInChunks(fromPlayer, parsedText, onSuccess)` - handles the extracted text
4. Callback in `AiBase.hx` just calls `myPlayer.say(response)` (text already parsed)

## Implementation Details

### File: `openlife/server/AiHandler.hx`

1. **Add new function** after `checkIfShouldDoCommand()`:
```haxe
private static function getCommandContext():String {
    return "You can also respond with a JSON command object to perform actions:
{ \"text\": \"your response\", \"command\": { \"type\": \"doEmote\", \"emote\": \"emoteName\" } }
Available emotes: happy, angry, love, sad, joy, blush, devious, shock, terrified, homesick, mad, oreally, ill, hmph, snowSplat";
}
```

2. **Modify `buildPrompt()`** to include command context before the user message.

3. **Add new function** for parsing and executing commands:
```haxe
private static function parseAiResponse(response:String, aiPlayer:GlobalPlayerInstance):String {
    var text = response;
    try {
        var json = haxe.Json.parse(response);
        if (Reflect.hasField(json, "text")) {
            text = json.text;
        }
        if (Reflect.hasField(json, "command")) {
            var command = json.command;
            if (Reflect.hasField(command, "type") && command.type == "doEmote") {
                var emoteName = command.emote;
                var emoteId = getEmoteId(emoteName);
                if (emoteId >= 0) {
                    aiPlayer.doEmote(emoteId);
                }
            }
        }
    } catch(e:Dynamic) {
        // Not valid JSON, use original response
    }
    return text;
}
```

4. **Add helper function** to get emote ID by name:
```haxe
private static function getEmoteId(emoteName:String):Int {
    switch(emoteName.toLowerCase()) {
        case "happy": return 0;
        case "mad": return 1;
        case "angry": return 2;
        case "sad": return 3;
        case "devious": return 4;
        case "joy": return 5;
        case "blush": return 6;
        case "snowSplat": return 8;
        case "oreally": return 14;
        case "ill": return 10;
        case "hmph": return 12;
        case "shock": return 15;
        case "love": return 13;
        case "terrified": return 27;
        case "homesick": return 28;
        default: return -1;
    }
}
```

5. **Modify `respondToPlayerAsync()`** in `AiHandler.hx` to call `parseAiResponse` before `sendResponseInChunks`:
```haxe
// Spawn a new thread to call the LLM without blocking the main thread
Thread.create(function() {
    // Call ChatResponse directly with the pre-built prompt
    var response = ChatResponse(fullPrompt);
    response = response.split("\n").join(" ");

    // Log the conversation to file (thread-safe)
    logToFile(fullPrompt, response);

    // Add to chat memory only if call succeeded
    if (response != null) {
        fromSoul.addChatEntry(toPlayer, message, response);
    }

    // Parse response to extract text and execute commands BEFORE sending
    var parsedText = parseAiResponse(response, fromPlayer);

    // Execute the callback with the parsed text
    sendResponseInChunks(fromPlayer, parsedText, onSuccess);
});
```

6. **Simplify the `onSuccess` callback in `AiBase.hx:4961`** to just say the response:
```haxe
AiHandler.respondToPlayerAsync(aiPlayer, player, text, function(response:String) {
    if (response != null) {
        myPlayer.say(response);
        // ... rest of existing code
    }
});
```

## Summary of Changes

| File | Changes |
|------|---------|
| `AiHandler.hx` | Add `getCommandContext()`, `parseAiResponse()`, `getEmoteId()` functions; modify `buildPrompt()` to include command context; call `parseAiResponse()` before `sendResponseInChunks()` in `respondToPlayerAsync()` |
| `AiBase.hx` | Simplify `respondToPlayerAsync()` callback - just calls `myPlayer.say(response)` since text is already parsed |
