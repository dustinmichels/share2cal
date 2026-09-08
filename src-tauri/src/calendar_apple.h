#ifndef CALENDAR_APPLE_H
#define CALENDAR_APPLE_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Returns permission status: "not_determined", "restricted", "denied", "authorized", "write_only", or "unknown"
int calendar_apple_check_permission(char **out_status, char **out_error);

// Requests write-only (or full) calendar access.
// Returns 0 on success (with *out_granted = 1 or 0), or non-zero on error.
int calendar_apple_request_permission(int *out_granted, char **out_error);

// Creates an event in the default Apple calendar.
// start_epoch and end_epoch are seconds since Unix epoch (1970-01-01 00:00:00 UTC).
// is_all_day: 1 for all-day event, 0 for timed event.
// title, location, notes, url can be NULL or UTF-8 strings.
// On success, returns 0 and sets *out_event_id to allocated C string (free with calendar_apple_free_string).
// On error, returns non-zero and sets *out_error to allocated C string.
int calendar_apple_create_event(
    const char *title,
    double start_epoch,
    double end_epoch,
    int is_all_day,
    const char *location,
    const char *notes,
    const char *url,
    const char *recurrence_rule,
    char **out_event_id,
    char **out_error
);

void calendar_apple_free_string(char *ptr);

#ifdef __cplusplus
}
#endif

#endif // CALENDAR_APPLE_H
