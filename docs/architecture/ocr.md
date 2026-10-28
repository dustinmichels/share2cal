# Reading text (OCR)

Text recognition runs entirely on the device through Apple's Vision framework. One call returns the recognised lines, their positions, and the contents of any QR codes.

## The Vision request

`ocr_apple.m` decodes the image with ImageIO (`CGImageSourceCreateWithData` for bytes, `CGImageSourceCreateWithURL` for paths) and runs two requests through a single `VNImageRequestHandler`:

| Request                   | Settings                                                      |
| ------------------------- | ------------------------------------------------------------- |
| `VNRecognizeTextRequest`  | `recognitionLevel = Accurate`, `usesLanguageCorrection = YES` |
| `VNDetectBarcodesRequest` | `symbologies = [QR]`                                          |

If the combined call fails, it retries with text recognition alone, so a barcode error never costs you the text.

For every text observation the bridge keeps the top candidate's string, its confidence and the observation's bounding box. The full `text` is those strings joined with newlines, in Vision's order. The whole result is serialised to JSON and returned as a C string, which `ocr.rs` parses into `OcrResult` and frees through `NativeStringGuard`.

```rust
pub struct OcrResult {
    pub text: String,
    pub lines: Vec<OcrLine>,     // text, confidence, bounding_box
    pub qr_codes: Vec<String>,   // raw QR payloads
}
```

Bounding boxes use Vision's normalised coordinates: 0 to 1 on both axes, with the origin at the **bottom**-left.

## Rebuilding rows

Vision returns text in reading blocks, not visual rows. On a timetable, the course code, the days and the room for one class can come back as three separate lines far apart in `text`. `reconstruct_spatial_lines` puts them back together:

1. Sort boxes top to bottom by vertical centre.
2. Assign each box to an existing row if it overlaps the row vertically by more than 35% of the smaller height, or its centre is within that distance. Otherwise start a new row. Row centre and height are running averages.
3. Sort rows top to bottom, sort boxes within a row left to right, and join each row with spaces.

The OCR commands only use the rebuilt text when it pays off. `extract_text_from_image_bytes` runs the schedule-table parser on it, and if that finds two or more events, the rebuilt text replaces `text` in the result. Otherwise the original Vision order is kept, which reads better for ordinary flyers.

## QR codes and links

QR payloads are returned untouched in `qr_codes`. Only payloads starting with `http://` or `https://` are treated as links (`is_web_link` in Rust, `isWebLink` in TypeScript), so Wi-Fi or contact QR codes do not become event URLs.

The frontend also scans the recognised text for URLs (`extractUrlsFromText`) and merges them with the QR links. The combined list is:

- appended to the text sent to the parser, under a `Links / QR Codes:` heading, so the parser and model can see it;
- used to fill `url` on any event that came back without one.

`generate_extraction_prompt` treats the `Links / QR Codes:` block specially: when the OCR text is too long for the prompt budget, the text is trimmed but the links block is always kept.

## Other platforms

Outside macOS and iOS, `ocr.rs` compiles a fallback whose functions return:

```text
AppError::Ocr("OCR is only supported on Apple platforms (macOS/iOS) or Android ML Kit.")
```

The ML Kit integration that message mentions does not exist yet. On those platforms, pasted text is the only working input.
