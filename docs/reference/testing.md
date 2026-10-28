# Testing

Two suites share one set of real flyers. Rust tests cover OCR, parsing, the model plumbing and the calendar logic; bun tests cover the TypeScript services.

## Ground truth

`samples/` holds seven real images, and `samples/parsed.json` lists the events a person would expect from each.

| Image                 | What it is                             | Expected events |
| --------------------- | -------------------------------------- | --------------- |
| `class.png`           | One course from a registration page    | 1, weekly       |
| `classes.png`         | Registration schedule table            | 6, weekly       |
| `commons.jpg`         | Campus talk flyer                      | 1               |
| `gilman_flyer.png`    | Street festival poster, phone photo    | 1               |
| `instagram.png`       | Instagram post for an ice cream social | 1               |
| `ride_for_life.png`   | Bike ride poster, phone photo          | 1               |
| `squirrel_flower.jpg` | Concert poster with a support act      | 1               |

Each expected event records `title`, `date` or `days`, `start_time` and `end_time` as wall-clock strings (`"1:20 PM"`), `is_all_day`, `repeating`, `location`, `description` and `url`. `parsed.md` is the same data in readable form.

Comparisons are deliberately loose: titles and locations are fuzzy-matched on significant tokens, times are normalised to 24-hour, and recurrence compares `BYDAY` sets. The helpers live in `src-tauri/tests/common/mod.rs`.

## Rust

```sh
cd src-tauri
cargo test
```

| Location                         | Covers                                                                                                                                                                          |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `#[cfg(test)]` modules in `src/` | Date, time and recurrence parsing; title, location and schedule heuristics; spatial row rebuilding; prompt budget and grammar; LLM JSON repair; epoch conversion; share staging |
| `tests/samples_validation.rs`    | OCRs every sample and checks the rule-based parser against `parsed.json`; checks every prompt fits the context budget; multi-event extraction from `classes.png` and the flyers |
| `tests/calendar_integration.rs`  | Permission commands, empty-title and bad-RRULE rejection, batch prevalidation, URL preservation, and that the mock is active so no real calendar is touched                     |
| `tests/share_integration.rs`     | Stage, read and clear a pending share; reject empty bytes; load from a path                                                                                                     |

Most sample tests are `#[cfg(any(target_os = "macos", target_os = "ios"))]` because they need Vision. On other hosts they are compiled out.

Calendar tests are safe to run on your own Mac. The mock activates automatically under `cfg(test)` and for any Cargo test binary, and the integration tests also force it on through `test_support::ensure_mock_calendar()`.

### Testing with the real model

`test_all_samples_llm_inference_enhanced_against_parsed_json` runs the actual SmolLM2 360M model on every sample. It is `#[ignore]`d because model output does not yet match the strict ground truth. To run it:

```sh
SHARE2CAL_MODELS_DIR=/path/to/models cargo test -- --ignored
```

The directory must contain `SmolLM2-360M-Instruct-Q4_K_M.gguf`. Without `SHARE2CAL_MODELS_DIR`, the test looks in `~/Library/Application Support/com.dustinmichels.share2cal/models/`, where the app itself downloads models.

## Frontend

```sh
bun test
```

`bunfig.toml` points the runner at `./tests`. These tests run outside Tauri, so anything that calls `invoke` exercises the browser fallback path.

| File               | Covers                                                                                                                                         |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `event.test.ts`    | `.ics` generation for every sample in `parsed.json`, single and multi-event; RRULE parse and build; date and time form helpers; URL extraction |
| `calendar.test.ts` | Google Calendar date formats, exclusive all-day end dates, URL building, calendar preference persistence                                       |
| `model.test.ts`    | Manifest shape, byte and speed formatting                                                                                                      |
| `settings.test.ts` | Defaults and round-trips for every stored preference                                                                                           |
| `share.test.ts`    | `payloadToFile` conversion and pending-share handling                                                                                          |

## Type checking and formatting

```sh
bun run build   # vue-tsc --noEmit, then vite build
bun run fmt     # oxfmt
```
