#import "ocr_apple.h"
#import <Foundation/Foundation.h>
#import <Vision/Vision.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>
#import <math.h>
static char *create_c_string(NSString *str) {
    if (!str) return NULL;
    const char *utf8 = [str UTF8String];
    return utf8 ? strdup(utf8) : NULL;
}

static int perform_ocr_on_source(CGImageSourceRef imageSource, char **out_json, char **out_error) {
    if (!imageSource) {
        if (out_error) *out_error = create_c_string(@"Failed to create image source.");
        return -1;
    }

    CGImageRef cgImage = CGImageSourceCreateImageAtIndex(imageSource, 0, NULL);
    if (!cgImage) {
        if (out_error) *out_error = create_c_string(@"Failed to decode image from source.");
        return -1;
    }

    @autoreleasepool {
        VNRecognizeTextRequest *request = [[VNRecognizeTextRequest alloc] init];
        request.recognitionLevel = VNRequestTextRecognitionLevelAccurate;
        request.usesLanguageCorrection = YES;

        VNImageRequestHandler *handler = [[VNImageRequestHandler alloc] initWithCGImage:cgImage options:@{}];
        CGImageRelease(cgImage);

        NSError *requestError = nil;
        BOOL success = [handler performRequests:@[request] error:&requestError];
        if (!success || requestError) {
            NSString *errDesc = requestError ? [requestError localizedDescription] : @"Vision text recognition request failed.";
            if (out_error) *out_error = create_c_string(errDesc);
            return -1;
        }

        NSArray<VNRecognizedTextObservation *> *observations = request.results;
        if (!observations) {
            observations = @[];
        }

        NSMutableArray<NSString *> *textLines = [NSMutableArray arrayWithCapacity:observations.count];
        NSMutableArray<NSDictionary *> *linesData = [NSMutableArray arrayWithCapacity:observations.count];

        for (VNRecognizedTextObservation *obs in observations) {
            VNRecognizedText *topCandidate = [[obs topCandidates:1] firstObject];
            if (topCandidate && topCandidate.string.length > 0) {
                NSString *str = topCandidate.string;
                [textLines addObject:str];

                CGRect box = obs.boundingBox;
                NSDictionary *boxDict = @{
                    @"x": @(box.origin.x),
                    @"y": @(box.origin.y),
                    @"width": @(box.size.width),
                    @"height": @(box.size.height)
                };

                NSDictionary *lineDict = @{
                    @"text": str,
                    @"confidence": @(topCandidate.confidence),
                    @"bounding_box": boxDict
                };
                [linesData addObject:lineDict];
            }
        }

        NSString *fullText = [textLines componentsJoinedByString:@"\n"];
        NSDictionary *resultDict = @{
            @"text": fullText,
            @"lines": linesData
        };

        NSError *jsonError = nil;
        NSData *jsonData = [NSJSONSerialization dataWithJSONObject:resultDict options:0 error:&jsonError];
        if (!jsonData || jsonError) {
            if (out_error) *out_error = create_c_string(@"Failed to serialize OCR result to JSON.");
            return -1;
        }

        NSString *jsonString = [[NSString alloc] initWithData:jsonData encoding:NSUTF8StringEncoding];
        if (out_json) {
            *out_json = create_c_string(jsonString);
        }
        return 0;
    }
}

int ocr_apple_from_file(const char *path, char **out_json, char **out_error) {
    if (!path) {
        if (out_error) *out_error = create_c_string(@"Path cannot be null.");
        return -1;
    }

    @autoreleasepool {
        NSString *nsPath = [NSString stringWithUTF8String:path];
        NSURL *url = [NSURL fileURLWithPath:nsPath];
        CGImageSourceRef imageSource = CGImageSourceCreateWithURL((__bridge CFURLRef)url, NULL);
        if (!imageSource) {
            if (out_error) *out_error = create_c_string([NSString stringWithFormat:@"Unable to open image at path: %@", nsPath]);
            return -1;
        }

        int res = perform_ocr_on_source(imageSource, out_json, out_error);
        CFRelease(imageSource);
        return res;
    }
}

int ocr_apple_from_bytes(const uint8_t *bytes, size_t length, char **out_json, char **out_error) {
    if (!bytes || length == 0) {
        if (out_error) *out_error = create_c_string(@"Image data cannot be empty.");
        return -1;
    }

    @autoreleasepool {
        NSData *nsData = [NSData dataWithBytes:bytes length:length];
        CGImageSourceRef imageSource = CGImageSourceCreateWithData((__bridge CFDataRef)nsData, NULL);
        if (!imageSource) {
            if (out_error) *out_error = create_c_string(@"Unable to parse image data.");
            return -1;
        }

        int res = perform_ocr_on_source(imageSource, out_json, out_error);
        CFRelease(imageSource);
        return res;
    }
}

void ocr_apple_free_string(char *ptr) {
    if (ptr) {
        free(ptr);
    }
}
