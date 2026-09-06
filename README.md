# Share2Cal

I am making a mobile app that allows users to share screenshots of text or images of flyers and automatically parse the text to create a calendar event. The app will use:

1. Native OCR (iOS: Apple Vision Framework, Android: Google ML Kit Text Recognition) to extract text from the image.
2. A tiny LLM to extract relevant information such as the event title, date, time, and location from the shared screenshot into a structured format to create a calendar event.
3. Native calendar APIs (iOS: EventKit, Android: Calendar Provider) to create the event in the user's calendar.

```sh
bun install
bun run tauri android init
bun run tauri ios init

# For Desktop development, run:
bun run tauri dev

# For Android development, run:
bun run tauri android dev

# For iOS development, run:
bun run tauri ios dev
```

## Dev

```sh
bun tauri dev

# android / apple
bun tauri android dev
bun tauri ios dev
```
