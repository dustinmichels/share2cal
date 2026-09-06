#ifndef OCR_APPLE_H
#define OCR_APPLE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int ocr_apple_from_file(const char *path, char **out_json, char **out_error);
int ocr_apple_from_bytes(const uint8_t *bytes, size_t length, char **out_json, char **out_error);
void ocr_apple_free_string(char *ptr);

#ifdef __cplusplus
}
#endif

#endif // OCR_APPLE_H
