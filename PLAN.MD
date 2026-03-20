# Plan: Split Long AI Responses

## Overview
Split AI responses exceeding `MaxAIResponseperSay` characters into multiple chunks, sent with delays to prevent overwhelming the client while keeping AI characters patient.

## Files to Modify

### 1. `openlife/settings/ServerSettings.hx`
Add two new static settings:
```hx
public static var MaxAIResponseperSay:Int = 100;
public static var AIWaitTimePer100Chars:Float = 10; // seconds per 100 characters
```

### 2. `openlife/auto/AiBase.hx`
Add a new instance method to set waiting time to a minimum:
```hx
public function setWaitingTimeMin(minTime:Float):Void {
    if (this.waitingTime < minTime) {
        this.waitingTime = minTime;
    }
}
```
**Note:** Remove `IncreaseWaitingTimeForAll()` - not needed.

### 3. `openlife/server/AiHandler.hx`
Modify `respondToPlayerAsync()` to pass `fromPlayer` into the thread so we can access the specific AI. Then in `sendResponseInChunks()`:
1. Get the specific AI via `fromPlayer.connection.serverAi.ai`
2. After receiving full response, check if length > `MaxAIResponseperSay` and contains split separators (`.!?*`)
3. If splitting needed:
   - Find the best split point (separator closest to but not exceeding `MaxAIResponseperSay`)
   - If no separator found within limit, use first separator after limit
   - Call `onSuccess` with first chunk immediately
   - Calculate wait time: `(totalChars / 100) * AIWaitTimePer100Chars` seconds
   - Call `aiBase.setWaitingTimeMin(totalWaitTime)` on the specific AI
   - `Sys.sleep(waitTime)` in thread
   - Repeat for remaining chunks (max 10 splits total)
4. If no splitting needed, call `onSuccess(response)` immediately as before

### 4. Revised AiHandler Implementation
```hx
var aiBase = fromPlayer.connection.serverAi.ai;
if (aiBase != null) {
    aiBase.setWaitingTimeMin(totalWaitTime);
}
```

## Implementation Details

### Splitting Algorithm
```
1. If response.length <= MaxAIResponseperSay OR no separator exists:
   - Call onSuccess(response) immediately
   
2. Otherwise:
   - Find all separator positions (.!?*)
   - Find separator closest to MaxAIResponseperSay from the left
   - If none found from left within limit, use closest from right (after limit)
   - Split at that position
   
3. For each chunk:
   - Call onSuccess(chunk)
   - If not last chunk:
     - Calculate waitTime = (originalResponse.length / 100) * AIWaitTimePer100Chars
     - Call aiBase.setWaitingTimeMin(waitTime)
     - Sys.sleep(waitTime)
```

### Timing Calculation (per user clarification)
- Example: 500 character response with AIWaitTimePer100Chars = 10
- Total wait time = (500 / 100) * 10 = 50 seconds
- Only the specific AI's `waitingTime` is set to 50 seconds if currently lower
- Other AIs are not affected

### Edge Cases
- No separator within limit: use first separator after limit
- Response with no separators: send as single chunk (no splitting)
- Max 10 splits: stop after 10 chunks even if response remains
- `fromPlayer.connection.serverAi` could be null if AI is not fully initialized: add null check

## Verification
- Compile with `haxe server.hxml`
- Test with long AI responses (>100 chars) and verify splitting
- Test that AI characters wait (not running away) during response chunks
- Test that short responses (<100 chars) work unchanged
