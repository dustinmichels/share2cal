#ifndef SHARE_APPLE_H
#define SHARE_APPLE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Returns a heap-allocated JSON string containing the pending share manifest or metadata, or NULL if none.
// Caller is responsible for freeing with free_share_string.
char* get_app_group_pending_share_json(const char* group_id);

// Returns a heap-allocated string containing the path to the latest shared image, or NULL if none.
// Caller is responsible for freeing with free_share_string.
char* get_app_group_shared_image_path(const char* group_id);

// Clears all pending shared images and manifests in the App Group container.
bool clear_app_group_shared_data(const char* group_id);

// Saves image bytes to the App Group container and updates the pending share manifest.
// Useful for staging, fallback, and simulator/testing environments.
bool save_app_group_shared_image(
    const char* group_id,
    const uint8_t* bytes,
    size_t len,
    const char* filename,
    const char* mime_type
);

// Frees a string returned by any share_apple functions.
void free_share_string(char* str);

#ifdef __cplusplus
}
#endif

#endif // SHARE_APPLE_H
