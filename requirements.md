# Requirements

I am making a native, cross-platform mobile app using Tauri 2 (bun, TS, vue, rust.)

The app allows users to share screenshots of text or images of flyers and automatically parse the text to create a calendar event. Requirements:

1. Receive an image via the share button
1. Native OCR (iOS: Apple Vision Framework, Android: Google ML Kit Text Recognition) to extract text from the image.
1. A tiny LLM to extract relevant information such as the event title, date, time, and location from the shared screenshot into a structured format to create a calendar event.
1. Native calendar APIs (iOS: EventKit, Android: Calendar Provider) to create the event in the user's calendar.
